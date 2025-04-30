use chrono::Utc;
use serde::Serialize;

use super::providers::get_network_provider;
use super::{HttpMethod, NetworkProvider, RequestArgs, Response};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::sdk_diagnostics::marker::{ActionType, Marker, StepType};
use crate::{log_d, log_i, log_w, StatsigErr, StatsigOptions};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

const RETRY_CODES: [u16; 8] = [408, 500, 502, 503, 504, 522, 524, 599];
const SHUTDOWN_ERROR: &str = "Request was aborted because the client is shutting down";

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum NetworkError {
    ShutdownError,
    RequestFailed,
    RetriesExhausted,
    SerializationError(String),
    DisableNetworkOn,
    RequestNotRetryable,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ShutdownError => write!(f, "ShutdownError"),
            NetworkError::RequestFailed => write!(f, "RequestFailed"),
            NetworkError::RetriesExhausted => write!(f, "RetriesExhausted"),
            NetworkError::SerializationError(s) => write!(f, "SerializationError: {s}"),
            NetworkError::DisableNetworkOn => write!(f, "DisableNetworkOn"),
            NetworkError::RequestNotRetryable => write!(f, "RequestNotRetryable"),
        }
    }
}

const TAG: &str = stringify!(NetworkClient);

pub struct NetworkClient {
    headers: HashMap<String, String>,
    is_shutdown: Arc<AtomicBool>,
    ops_stats: Arc<OpsStatsForInstance>,
    net_provider: Weak<dyn NetworkProvider>,
    disable_network: bool,
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
        let disable_network: bool = options
            .and_then(|opts| opts.disable_network)
            .unwrap_or(false);

        NetworkClient {
            headers: headers.unwrap_or_default(),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            net_provider,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            disable_network,
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
            return Err(NetworkError::DisableNetworkOn);
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
                return Err(NetworkError::ShutdownError);
            }

            let response = match self.net_provider.upgrade() {
                Some(net_provider) => net_provider.send(&method, &request_args).await,
                None => return Err(NetworkError::RequestFailed),
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
                .unwrap_or_else(|| get_error_message_for_status(status));

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

            if !RETRY_CODES.contains(&status) {
                let msg = format!("Network error, not retrying: {} {}", status, error_message);
                self.log_warning(
                    StatsigErr::NetworkError(NetworkError::RequestNotRetryable, Some(msg)),
                    &request_args,
                );
                return Err(NetworkError::RequestNotRetryable);
            }

            if attempt >= request_args.retries {
                let msg = format!(
                    "Network error, retries exhausted: {} {}",
                    status, error_message
                );
                self.log_warning(
                    StatsigErr::NetworkError(NetworkError::RetriesExhausted, Some(msg)),
                    &request_args,
                );
                return Err(NetworkError::RetriesExhausted);
            }

            attempt += 1;
            let backoff_ms = 2_u64.pow(attempt) * 100;

            log_w!(
                TAG, "Network request failed with status code {} (attempt {}), will retry after {}ms...\n{}",
                status,
                attempt,
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

    fn log_warning(&self, error: StatsigErr, args: &RequestArgs) {
        log_w!(TAG, "{}", error);
        if !self.silent_on_network_failure {
            let dedupe_key = format!("{:?}", args.diagnostics_key);
            self.ops_stats.log_error(ErrorBoundaryEvent {
                tag: TAG.to_string(),
                bypass_dedupe: false,
                info: error,
                dedupe_key: Some(dedupe_key),
                extra: None,
            });
        }
    }
}

fn get_error_message_for_status(status: u16) -> String {
    if (200..300).contains(&status) {
        return String::new();
    }

    match status {
        400 => "Bad Request".to_string(),
        401 => "Unauthorized".to_string(),
        403 => "Forbidden".to_string(),
        404 => "Not Found".to_string(),
        405 => "Method Not Allowed".to_string(),
        406 => "Not Acceptable".to_string(),
        408 => "Request Timeout".to_string(),
        500 => "Internal Server Error".to_string(),
        502 => "Bad Gateway".to_string(),
        503 => "Service Unavailable".to_string(),
        504 => "Gateway Timeout".to_string(),
        0 => "Unknown Error".to_string(),
        _ => format!("HTTP Error {status}"),
    }
}
