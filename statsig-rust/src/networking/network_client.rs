use chrono::Utc;

use super::network_error::NetworkError;
use super::providers::get_network_provider;
use super::{HttpMethod, NetworkProvider, RequestArgs, Response};
use crate::networking::proxy_config::ProxyConfig;
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::sdk_diagnostics::marker::{ActionType, Marker, StepType};
use crate::{log_d, log_i, log_w, StatsigOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

const NON_RETRY_CODES: [u16; 4] = [400, 403, 405, 501];
const SHUTDOWN_ERROR: &str = "Request was aborted because the client is shutting down";

const TAG: &str = stringify!(NetworkClient);

pub struct NetworkClient {
    headers: HashMap<String, String>,
    is_shutdown: Arc<AtomicBool>,
    ops_stats: Arc<OpsStatsForInstance>,
    net_provider: Weak<dyn NetworkProvider>,
    disable_network: bool,
    proxy_config: Option<ProxyConfig>,
    silent_on_network_failure: bool,
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

            let response = match self.net_provider.upgrade() {
                Some(net_provider) => net_provider.send(&method, &request_args).await,
                None => {
                    return Err(NetworkError::RequestFailed(
                        request_args.url,
                        0,
                        "Failed to get a NetworkProvider instance".to_string(),
                    ));
                }
            };

            log_d!(
                TAG,
                "Response ({}): {}",
                &request_args.url,
                response.status_code
            );

            let status = response.status_code;
            let sdk_region_str = response
                .headers
                .as_ref()
                .and_then(|h| h.get("x-statsig-region"));
            let success = (200..300).contains(&status);

            let error_message = response
                .error
                .clone()
                .unwrap_or_else(|| get_error_message_for_status(status, response.data.as_deref()));

            if let Some(key) = request_args.diagnostics_key {
                let mut end_marker =
                    Marker::new(key, ActionType::End, Some(StepType::NetworkRequest))
                        .with_attempt(attempt)
                        .with_url(request_args.url.clone())
                        .with_status_code(status)
                        .with_is_success(success)
                        .with_sdk_region(sdk_region_str.map(|s| s.to_owned()));

                let error_map = if !error_message.is_empty() {
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), "NetworkError".to_string());
                    map.insert("message".to_string(), error_message.clone());
                    map.insert("code".to_string(), status.to_string());
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

            if NON_RETRY_CODES.contains(&status) {
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
                status,
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
}

fn get_error_message_for_status(status: u16, data: Option<&[u8]>) -> String {
    if (200..300).contains(&status) {
        return String::new();
    }

    let mut message = String::new();
    if let Some(data) = data {
        let lossy_str = String::from_utf8_lossy(data);
        if lossy_str.is_ascii() {
            message = lossy_str.to_string();
        }
    }

    let generic_message = match status {
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
        _ => return format!("HTTP Error {status}: {message}"),
    };

    if message.is_empty() {
        return generic_message.to_string();
    }

    format!("{generic_message}: {message}")
}
