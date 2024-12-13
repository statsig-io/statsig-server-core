use crate::{log_e, log_i, log_w};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;

use super::{Curl, HttpMethod, RequestArgs};

const RETRY_CODES: [u16; 8] = [408, 500, 502, 503, 504, 522, 524, 599];
const SHUTDOWN_ERROR: &str = "Request was aborted because the client is shutting down";

const TAG: &str = stringify!(NetworkClient);

pub struct NetworkClient {
    headers: HashMap<String, String>,
    is_shutdown: Arc<AtomicBool>,
    curl: Curl,
}

impl NetworkClient {
    pub fn new(sdk_key: &str, headers: Option<HashMap<String, String>>) -> Self {
        NetworkClient {
            headers: headers.unwrap_or_default(),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            curl: Curl::get(sdk_key),
        }
    }

    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    pub async fn get(&self, request_args: RequestArgs) -> Option<String> {
        self.make_request(HttpMethod::GET, request_args).await
    }

    pub async fn post(&self, mut request_args: RequestArgs, body: Option<Bytes>) -> Option<String> {
        request_args.body = body;
        self.make_request(HttpMethod::POST, request_args).await
    }

    async fn make_request(
        &self,
        method: HttpMethod,
        mut request_args: RequestArgs,
    ) -> Option<String> {
        let is_shutdown = if let Some(is_shutdown) = &request_args.is_shutdown {
            is_shutdown.clone()
        } else {
            self.is_shutdown.clone()
        };

        if !self.headers.is_empty() {
            let mut merged_headers = request_args.headers.unwrap_or_default();
            merged_headers.extend(self.headers.clone());
            request_args.headers = Some(merged_headers);
        }

        let mut attempt = 0;

        loop {
            if is_shutdown.load(Ordering::SeqCst) {
                log_i!(TAG, "{}", SHUTDOWN_ERROR);
                return None;
            }

            let response = self.curl.send(&method, &request_args).await;

            let status = response.status_code;

            if (200..300).contains(&status) {
                return response.data;
            }

            let error_message = response
                .error
                .unwrap_or_else(|| get_error_message_for_status(status));

            if !RETRY_CODES.contains(&status) {
                log_e!(TAG, "NetworkClient Error: {} {}", status, error_message);
                return None;
            }

            if attempt >= request_args.retries {
                log_e!(
                    TAG,
                    "Network error, retries exhausted: {} {}",
                    status,
                    error_message
                );
                return None;
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
        _ => format!("HTTP Error {}", status),
    }
}
