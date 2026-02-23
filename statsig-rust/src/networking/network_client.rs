use chrono::Utc;

use super::network_error::NetworkError;
use super::providers::get_network_provider;
use super::{HttpMethod, NetworkProvider, RequestArgs, Response};
use crate::networking::proxy_config::ProxyConfig;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::sdk_diagnostics::marker::{ActionType, Marker, StepType};
use crate::utils::{is_version_segment, split_host_and_path, strip_query_and_fragment};
use crate::{log_d, log_i, log_w, StatsigOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

const NON_RETRY_CODES: [u16; 6] = [
    400, // Bad Request
    403, // Forbidden
    413, // Payload Too Large
    405, // Method Not Allowed
    429, // Too Many Requests
    501, // Not Implemented
];
const SHUTDOWN_ERROR: &str = "Request was aborted because the client is shutting down";

const MAX_REQUEST_PATH_LENGTH: usize = 64;
const LOGGABLE_KEY_PREFIX_LENGTH: usize = 13;
const DOWNLOAD_CONFIG_SPECS_ENDPOINT: &str = "download_config_specs";
const GET_ID_LISTS_ENDPOINT: &str = "get_id_lists";
const DOWNLOAD_ID_LIST_FILE_ENDPOINT: &str = "download_id_list_file";
const NETWORK_REQUEST_LATENCY_METRIC: &str = "network_request.latency";
const REQUEST_PATH_TAG: &str = "request_path";
const STATUS_CODE_TAG: &str = "status_code";
const IS_SUCCESS_TAG: &str = "is_success";
const SDK_KEY_TAG: &str = "sdk_key";
const SOURCE_SERVICE_TAG: &str = "source_service";
const ID_LIST_FILE_ID_TAG: &str = "id_list_file_id";

const TAG: &str = stringify!(NetworkClient);

pub struct NetworkClient {
    headers: HashMap<String, String>,
    is_shutdown: Arc<AtomicBool>,
    ops_stats: Arc<OpsStatsForInstance>,
    net_provider: Weak<dyn NetworkProvider>,
    disable_network: bool,
    proxy_config: Option<ProxyConfig>,
    silent_on_network_failure: bool,
    disable_file_streaming: bool,
    loggable_sdk_key: String,
}

impl NetworkClient {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        headers: Option<HashMap<String, String>>,
        options: Option<&StatsigOptions>,
    ) -> Self {
        let net_provider = get_network_provider();
        let (disable_network, proxy_config) = options
            .map(|opts| {
                (
                    opts.disable_network.unwrap_or(false),
                    opts.proxy_config.clone(),
                )
            })
            .unwrap_or((false, None));

        NetworkClient {
            headers: headers.unwrap_or_default(),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            net_provider,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            disable_network,
            proxy_config,
            silent_on_network_failure: false,
            disable_file_streaming: options
                .map(|opts| opts.disable_disk_access.unwrap_or(false))
                .unwrap_or(false),
            loggable_sdk_key: get_loggable_sdk_key(sdk_key),
        }
    }

    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    pub async fn get(&self, request_args: RequestArgs) -> Result<Response, NetworkError> {
        self.make_request(HttpMethod::GET, request_args).await
    }

    pub async fn post(
        &self,
        mut request_args: RequestArgs,
        body: Option<Vec<u8>>,
    ) -> Result<Response, NetworkError> {
        request_args.body = body;
        self.make_request(HttpMethod::POST, request_args).await
    }

    async fn make_request(
        &self,
        method: HttpMethod,
        mut request_args: RequestArgs,
    ) -> Result<Response, NetworkError> {
        let is_shutdown = if let Some(is_shutdown) = &request_args.is_shutdown {
            is_shutdown.clone()
        } else {
            self.is_shutdown.clone()
        };

        if self.disable_network {
            log_d!(TAG, "Network is disabled, not making requests");
            return Err(NetworkError::DisableNetworkOn(request_args.url));
        }

        request_args.populate_headers(self.headers.clone());

        if request_args.disable_file_streaming.is_none() {
            request_args.disable_file_streaming = Some(self.disable_file_streaming);
        }

        let mut merged_headers = request_args.headers.unwrap_or_default();
        if !self.headers.is_empty() {
            merged_headers.extend(self.headers.clone());
        }
        merged_headers.insert(
            "STATSIG-CLIENT-TIME".into(),
            Utc::now().timestamp_millis().to_string(),
        );
        request_args.headers = Some(merged_headers);

        // passing down proxy config through request args
        if let Some(proxy_config) = &self.proxy_config {
            request_args.proxy_config = Some(proxy_config.clone());
        }
        let mut attempt = 0;

        loop {
            if let Some(key) = request_args.diagnostics_key {
                self.ops_stats.add_marker(
                    Marker::new(key, ActionType::Start, Some(StepType::NetworkRequest))
                        .with_attempt(attempt)
                        .with_url(request_args.url.clone()),
                    None,
                );
            }
            if is_shutdown.load(Ordering::SeqCst) {
                log_i!(TAG, "{}", SHUTDOWN_ERROR);
                return Err(NetworkError::ShutdownError(request_args.url));
            }

            let request_start = Instant::now();
            let mut response = match self.net_provider.upgrade() {
                Some(net_provider) => net_provider.send(&method, &request_args).await,
                None => {
                    return Err(NetworkError::RequestFailed(
                        request_args.url,
                        None,
                        "Failed to get a NetworkProvider instance".to_string(),
                    ));
                }
            };

            let status = response.status_code;
            let error_message = response
                .error
                .clone()
                .unwrap_or_else(|| get_error_message_for_status(status, response.data.as_mut()));

            let content_type = response
                .data
                .as_ref()
                .and_then(|data| data.get_header_ref("content-type"));

            log_d!(
                TAG,
                "Response url({}) status({:?}) content-type({:?})",
                &request_args.url,
                response.status_code,
                content_type
            );

            let sdk_region_str = response
                .data
                .as_ref()
                .and_then(|data| data.get_header_ref("x-statsig-region").cloned());
            let success = (200..300).contains(&status.unwrap_or(0));
            let duration_ms = request_start.elapsed().as_millis() as f64;
            self.log_network_request_latency_to_ob(&request_args, status, success, duration_ms);

            if let Some(key) = request_args.diagnostics_key {
                let mut end_marker =
                    Marker::new(key, ActionType::End, Some(StepType::NetworkRequest))
                        .with_attempt(attempt)
                        .with_url(request_args.url.clone())
                        .with_is_success(success)
                        .with_content_type(content_type.cloned())
                        .with_sdk_region(sdk_region_str.map(|s| s.to_owned()));

                if let Some(status_code) = status {
                    end_marker = end_marker.with_status_code(status_code);
                }

                let error_map = if !error_message.is_empty() {
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), "NetworkError".to_string());
                    map.insert("message".to_string(), error_message.clone());
                    let status_string = match status {
                        Some(code) => code.to_string(),
                        None => "None".to_string(),
                    };
                    map.insert("code".to_string(), status_string);
                    Some(map)
                } else {
                    None
                };

                if let Some(error_map) = error_map {
                    end_marker = end_marker.with_error(error_map);
                }

                self.ops_stats.add_marker(end_marker, None);
            }

            if success {
                return Ok(response);
            }

            if NON_RETRY_CODES.contains(&status.unwrap_or(0)) {
                let error = NetworkError::RequestNotRetryable(
                    request_args.url.clone(),
                    status,
                    error_message,
                );
                self.log_warning(&error, &request_args);
                return Err(error);
            }

            if attempt >= request_args.retries {
                let error = NetworkError::RetriesExhausted(
                    request_args.url.clone(),
                    status,
                    attempt + 1,
                    error_message,
                );
                self.log_warning(&error, &request_args);
                return Err(error);
            }

            attempt += 1;
            let backoff_ms = 2_u64.pow(attempt) * 100;

            log_i!(
                TAG, "Network request failed with status code {} (attempt {}/{}), will retry after {}ms...\n{}",
                status.map_or("unknown".to_string(), |s| s.to_string()),
                attempt,
                request_args.retries + 1,
                backoff_ms,
                error_message
            );

            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    }

    pub fn mute_network_error_log(mut self) -> Self {
        self.silent_on_network_failure = true;
        self
    }

    // Logging helpers
    fn log_warning(&self, error: &NetworkError, args: &RequestArgs) {
        let exception = error.name();

        log_w!(TAG, "{}", error);
        if !self.silent_on_network_failure {
            let dedupe_key = format!("{:?}", args.diagnostics_key);
            self.ops_stats.log_error(ErrorBoundaryEvent {
                tag: TAG.to_string(),
                exception: exception.to_string(),
                bypass_dedupe: false,
                info: serde_json::to_string(error).unwrap_or_default(),
                dedupe_key: Some(dedupe_key),
                extra: None,
            });
        }
    }

    // ------------------------------------------------------------
    // Observability Logging Helpers (OB only) - START
    // ------------------------------------------------------------
    fn log_network_request_latency_to_ob(
        &self,
        request_args: &RequestArgs,
        status: Option<u16>,
        success: bool,
        duration_ms: f64,
    ) {
        let url = request_args.url.as_str();
        if !should_log_network_request_latency(url) {
            return;
        }

        let status_code = status
            .map(|code| code.to_string())
            .unwrap_or("none".to_string());
        let tags = get_network_request_latency_tags(
            request_args,
            status_code,
            success,
            self.loggable_sdk_key.clone(),
        );

        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            NETWORK_REQUEST_LATENCY_METRIC.to_string(),
            duration_ms,
            Some(tags),
        ));
    }
}

fn get_network_request_latency_tags(
    request_args: &RequestArgs,
    status_code: String,
    success: bool,
    loggable_sdk_key: String,
) -> HashMap<String, String> {
    let (source_service, request_path) = get_source_service_and_request_path(&request_args.url);
    let mut tags = HashMap::from([
        (REQUEST_PATH_TAG.to_string(), request_path),
        (STATUS_CODE_TAG.to_string(), status_code),
        (IS_SUCCESS_TAG.to_string(), success.to_string()),
        (SDK_KEY_TAG.to_string(), loggable_sdk_key),
        (SOURCE_SERVICE_TAG.to_string(), source_service),
    ]);
    if let Some(id_list_file_id) = request_args
        .id_list_file_id
        .as_ref()
        .filter(|id| !id.is_empty())
    {
        tags.insert(ID_LIST_FILE_ID_TAG.to_string(), id_list_file_id.clone());
    }

    tags
}

fn get_loggable_sdk_key(sdk_key: &str) -> String {
    sdk_key.chars().take(LOGGABLE_KEY_PREFIX_LENGTH).collect()
}

fn is_latency_loggable_endpoint(endpoint: &str) -> bool {
    endpoint == DOWNLOAD_CONFIG_SPECS_ENDPOINT
        || endpoint == GET_ID_LISTS_ENDPOINT
        || endpoint == DOWNLOAD_ID_LIST_FILE_ENDPOINT
}

fn get_version_and_endpoint_for_latency<'a>(
    segments: &'a [&'a str],
) -> Option<(usize, &'a str, &'a str)> {
    // Find a known endpoint pattern, then verify the segment right before it is `/v{number}`.
    segments
        .iter()
        .enumerate()
        .find_map(|(endpoint_index, endpoint_segment)| {
            if !is_latency_loggable_endpoint(endpoint_segment) || endpoint_index == 0 {
                return None;
            }

            let version_index = endpoint_index - 1;
            let version_segment = segments[version_index];
            is_version_segment(version_segment).then_some((
                version_index,
                version_segment,
                *endpoint_segment,
            ))
        })
}

fn should_log_network_request_latency(url: &str) -> bool {
    let (_, raw_path) = split_host_and_path(url);
    let normalized_path = strip_query_and_fragment(raw_path).trim_start_matches('/');
    let segments: Vec<&str> = normalized_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    get_version_and_endpoint_for_latency(&segments).is_some()
}

fn with_host_prefix(host_prefix: &str, path: &str) -> String {
    if host_prefix.is_empty() {
        path.to_string()
    } else {
        format!("{host_prefix}{path}")
    }
}

fn get_source_service_and_request_path(url: &str) -> (String, String) {
    let (host_prefix, raw_path) = split_host_and_path(url);
    let normalized_path = strip_query_and_fragment(raw_path).trim_start_matches('/');
    let segments: Vec<&str> = normalized_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    if let Some((version_index, version_segment, endpoint_segment)) =
        get_version_and_endpoint_for_latency(&segments)
    {
        let request_path = format!("/{version_segment}/{endpoint_segment}");
        let source_service_suffix = segments[..version_index].join("/");
        let source_service = with_host_prefix(&host_prefix, &source_service_suffix)
            .trim_end_matches('/')
            .to_string();
        return (source_service, request_path);
    }

    let fallback_request_path: String = normalized_path
        .chars()
        .take(MAX_REQUEST_PATH_LENGTH)
        .collect();
    let request_path = if fallback_request_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{fallback_request_path}")
    };
    let source_service = host_prefix.trim_end_matches('/').to_string();
    (source_service, request_path)
}

#[cfg(test)]
fn get_request_path(url: &str) -> String {
    get_source_service_and_request_path(url).1
}

// ------------------------------------------------------------
// Observability Logging Helpers (OB only) - END
// ------------------------------------------------------------

fn get_error_message_for_status(
    status: Option<u16>,
    data: Option<&mut super::ResponseData>,
) -> String {
    if (200..300).contains(&status.unwrap_or(0)) {
        return String::new();
    }

    let mut message = String::new();
    if let Some(data) = data {
        let lossy_str = data.read_to_string().unwrap_or_default();
        if lossy_str.is_ascii() {
            message = lossy_str.to_string();
        }
    }

    let status_value = match status {
        Some(code) => code,
        None => return format!("HTTP Error None: {message}"),
    };

    let generic_message = match status_value {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        408 => "Request Timeout",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        0 => "Unknown Error",
        _ => return format!("HTTP Error {status_value}: {message}"),
    };

    if message.is_empty() {
        return generic_message.to_string();
    }

    format!("{generic_message}: {message}")
}

#[cfg(test)]
mod tests {
    use super::{
        get_network_request_latency_tags, get_request_path, get_source_service_and_request_path,
        should_log_network_request_latency, ID_LIST_FILE_ID_TAG, REQUEST_PATH_TAG,
    };
    use crate::networking::RequestArgs;

    #[test]
    fn test_get_request_path_with_sample_urls() {
        assert_eq!(
            get_request_path("https://api.statsigcdn.com/v1/download_id_list_file/3wHgh0FhoQH0p"),
            "/v1/download_id_list_file"
        );
        assert_eq!(
            get_request_path("https://api.statsigcdn.com/v1/download_id_list_file/Q9mXcz7L1P43tRb8kV2dHyw%2FM6nJf0Ae5uTqsrC4Gp9KZ?foo=bar"),
            "/v1/download_id_list_file"
        );
        assert_eq!(
            get_request_path("https://api.statsig.com/v1/get_id_lists/secret-abcdef"),
            "/v1/get_id_lists"
        );
        assert_eq!(
            get_request_path("https://api.statsigcdn.com/v2/download_config_specs/secret-123456"),
            "/v2/download_config_specs"
        );
    }

    #[test]
    fn test_should_log_network_request_latency_for_supported_endpoints() {
        assert!(!should_log_network_request_latency(
            "https://api.statsig.com/v1/log_event"
        ));
        assert!(!should_log_network_request_latency(
            "https://api.statsig.com/v1/sdk_exception"
        ));
        assert!(should_log_network_request_latency(
            "https://api.statsig.com/v1/get_id_lists/secret-abcdef"
        ));
        assert!(should_log_network_request_latency(
            "https://api.statsigcdn.com/v2/download_config_specs/secret-123456"
        ));
        assert!(should_log_network_request_latency(
            "https://api.statsigcdn.com/v1/download_id_list_file/3wHgh0FhoQH0p"
        ));
    }

    #[test]
    fn test_get_source_service_and_request_path() {
        let (source_service, request_path) = get_source_service_and_request_path(
            "http://127.0.0.1:12345/mock-uuid/v2/download_config_specs/secret-key.json?x=1",
        );
        assert_eq!(source_service, "http://127.0.0.1:12345/mock-uuid");
        assert_eq!(request_path, "/v2/download_config_specs");
    }

    #[test]
    fn test_network_latency_tags_include_id_list_file_id_only_when_present() {
        let mut request_args = RequestArgs {
            url: "https://api.statsigcdn.com/v1/download_id_list_file/file-123".to_string(),
            id_list_file_id: Some("file-123".to_string()),
            ..RequestArgs::new()
        };

        let tags = get_network_request_latency_tags(
            &request_args,
            "200".to_string(),
            true,
            "client-key-123".to_string(),
        );
        assert_eq!(tags.get(ID_LIST_FILE_ID_TAG), Some(&"file-123".to_string()));
        assert_eq!(
            tags.get(REQUEST_PATH_TAG),
            Some(&"/v1/download_id_list_file".to_string())
        );

        request_args.id_list_file_id = Some(String::new());
        let tags_without_id = get_network_request_latency_tags(
            &request_args,
            "200".to_string(),
            true,
            "client-key-123".to_string(),
        );
        assert!(!tags_without_id.contains_key(ID_LIST_FILE_ID_TAG));
    }
}
