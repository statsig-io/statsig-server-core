use chrono::Utc;

use super::providers::get_network_provider;
use super::{HttpMethod, NetworkProvider, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::sdk_diagnostics::marker::{ActionType, Marker, StepType};
use crate::{log_error_to_statsig_and_console, log_i, log_w};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const RETRY_CODES: [u16; 8] = [408, 500, 502, 503, 504, 522, 524, 599];
const SHUTDOWN_ERROR: &str = "Request was aborted because the client is shutting down";

#[derive(PartialEq, Debug)]
pub enum NetworkError {
    ShutdownError,
    RequestFailed,
    RetriesExhausted,
    SerializationError,
}
const TAG: &str = stringify!(NetworkClient);

pub struct NetworkClient {
    headers: HashMap<String, String>,
    is_shutdown: Arc<AtomicBool>,
    ops_stats: Arc<OpsStatsForInstance>,

    #[cfg(feature = "custom_network_provider")]
    net_provider: std::sync::Weak<dyn NetworkProvider>,
    #[cfg(not(feature = "custom_network_provider"))]
    net_provider: Arc<dyn NetworkProvider>,
}

impl NetworkClient {
    #[must_use]
    pub fn new(sdk_key: &str, headers: Option<HashMap<String, String>>) -> Self {
        let net_provider = get_network_provider(sdk_key);

        NetworkClient {
            headers: headers.unwrap_or_default(),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            net_provider,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        }
    }

    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    pub async fn get(&self, request_args: RequestArgs) -> Result<String, NetworkError> {
        self.make_request(HttpMethod::GET, request_args).await
    }

    pub async fn post(
        &self,
        mut request_args: RequestArgs,
        body: Option<Vec<u8>>,
    ) -> Result<String, NetworkError> {
        request_args.body = body;
        self.make_request(HttpMethod::POST, request_args).await
    }

    async fn make_request(
        &self,
        method: HttpMethod,
        mut request_args: RequestArgs,
    ) -> Result<String, NetworkError> {
        let is_shutdown = if let Some(is_shutdown) = &request_args.is_shutdown {
            is_shutdown.clone()
        } else {
            self.is_shutdown.clone()
        };

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

            #[cfg(feature = "custom_network_provider")]
            let response = match self.net_provider.upgrade() {
                Some(net_provider) => net_provider.send(&method, &request_args).await,
                None => return Err(NetworkError::RequestFailed),
            };
            #[cfg(not(feature = "custom_network_provider"))]
            let response = self.net_provider.send(&method, &request_args).await;

            let status = response.status_code;
            let sdk_region_str = response
                .headers
                .as_ref()
                .and_then(|h| h.get("x-statsig-region"));
            let success = (200..300).contains(&status);

            let error_message = response
                .error
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
                return get_data_as_string(response.data);
            }

            if !RETRY_CODES.contains(&status) {
                log_error_to_statsig_and_console!(
                    &self.ops_stats,
                    TAG,
                    "status:{} message:{}",
                    status,
                    error_message
                );
                return Err(NetworkError::RequestFailed);
            }

            if attempt >= request_args.retries {
                log_error_to_statsig_and_console!(
                    &self.ops_stats,
                    TAG,
                    "Network error, retries exhausted: {} {}",
                    status,
                    error_message
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

fn get_data_as_string(data: Option<Vec<u8>>) -> Result<String, NetworkError> {
    // todo: support compressed data
    match data {
        Some(data) => Ok(String::from_utf8(data).map_err(|_| NetworkError::SerializationError)?),
        None => Err(NetworkError::RequestFailed),
    }
}
