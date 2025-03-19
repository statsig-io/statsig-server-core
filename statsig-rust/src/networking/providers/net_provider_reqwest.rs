use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use crate::{
    log_w,
    networking::{
        http_types::{HttpMethod, RequestArgs, Response},
        net_utils::sanitize_url_for_logging,
        NetworkProvider,
    },
    StatsigErr,
};

use reqwest::Method;

const TAG: &str = "NetworkProviderReqwest";

pub struct NetworkProviderReqwest {}

#[async_trait]
impl NetworkProvider for NetworkProviderReqwest {
    async fn send(&self, method: &HttpMethod, args: &RequestArgs) -> Response {
        if let Some(is_shutdown) = &args.is_shutdown {
            if is_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                return Response {
                    status_code: 0,
                    data: None,
                    error: Some("Request was shutdown".to_string()),
                    headers: None,
                };
            }
        }

        let sanitized_url = sanitize_url_for_logging(&args.url);
        let request = self.build_request(method, args);

        let error;
        let mut status_code = 0;
        let mut data = None;
        let mut headers = None;

        match request.send().await {
            Ok(response) => {
                status_code = response.status().as_u16();
                headers = get_response_headers(&response);
                data = response.bytes().await.ok().map(|bytes| bytes.to_vec());
                error = None;
            }
            Err(e) => {
                let error_message = get_error_message(e);
                log_w!(TAG, "Request Error: {} {}", sanitized_url, error_message);
                error = Some(error_message);
            }
        }

        Response {
            status_code,
            data,
            error,
            headers,
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }
}

impl NetworkProviderReqwest {
    fn build_request(
        &self,
        method: &HttpMethod,
        request_args: &RequestArgs,
    ) -> reqwest::RequestBuilder {
        let method_actual = match method {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
        };
        let is_post = method_actual == Method::POST;

        let client = reqwest::Client::new();
        let mut request = client.request(method_actual, &request_args.url);

        let timeout_duration = match request_args.timeout_ms > 0 {
            true => Duration::from_millis(request_args.timeout_ms),
            false => Duration::from_secs(10),
        };
        request = request.timeout(timeout_duration);

        if let Some(headers) = &request_args.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if let Some(params) = &request_args.query_params {
            request = request.query(params);
        }

        if is_post {
            let bytes = match &request_args.body {
                Some(b) => b.clone().into(),
                None => vec![],
            };
            let byte_len = bytes.len();

            request = request.body(bytes);
            request = request.header("Content-Length", byte_len.to_string());
        }

        request
    }
}

fn get_error_message(error: reqwest::Error) -> String {
    let mut error_message = error.to_string();

    if let Some(url_error) = error.url() {
        error_message.push_str(&format!(". URL: {}", url_error));
    }

    if let Some(status_error) = error.status() {
        error_message.push_str(&format!(". Status: {}", status_error));
    }

    error_message
}

fn get_response_headers(response: &reqwest::Response) -> Option<HashMap<String, String>> {
    let headers = response.headers();
    if headers.is_empty() {
        return None;
    }

    let mut headers_map = HashMap::new();
    for (key, value) in headers {
        headers_map.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
    }

    Some(headers_map)
}
