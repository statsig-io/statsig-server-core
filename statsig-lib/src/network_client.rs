use crate::{log_e, log_i, log_w};
use curl::easy::{Easy, List};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct NetworkClient {
    headers: HashMap<String, String>,
    retry_codes: HashSet<u16>,
}

pub struct RequestArgs {
    pub url: String,
    pub body: Option<HashMap<String, Value>>,
    pub retries: u32,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
}

impl RequestArgs {
    pub fn new() -> Self {
        RequestArgs {
            url: "".to_string(),
            body: None,
            retries: 0,
            headers: None,
            query_params: None,
        }
    }
}

#[derive(PartialEq)]
enum HttpMethod {
    GET,
    POST,
}

struct SimpleResponse {
    status_code: u16,
    data: Option<String>,
    error: Option<String>,
}

impl NetworkClient {
    pub fn new(headers: Option<HashMap<String, String>>) -> Self {
        NetworkClient {
            headers: headers.unwrap_or_default(),
            retry_codes: HashSet::from([408, 500, 502, 503, 504, 522, 524, 599]),
        }
    }

    pub fn get(&self, request_args: RequestArgs) -> Option<String> {
        self.execute_retryable_request(HttpMethod::GET, &request_args)
            .data
    }

    pub fn post(&self, request_args: RequestArgs) -> Option<String> {
        self.execute_retryable_request(HttpMethod::POST, &request_args)
            .data
    }

    fn execute_retryable_request(
        &self,
        method: HttpMethod,
        request_args: &RequestArgs,
    ) -> SimpleResponse {
        let mut attempt = 0;

        loop {
            let response = self.execute_single_request(&method, request_args);
            let status = response.status_code;

            if status >= 200 && status < 300 {
                return response;
            }

            let error_message = response.error.unwrap_or("Unknown Error".to_string());

            if !self.retry_codes.contains(&response.status_code) {
                log_e!("Network error: {}", error_message);
                return SimpleResponse {
                    error: Some(error_message),
                    ..response
                };
            }

            if attempt >= request_args.retries {
                log_e!("Network error, retries exhausted: {}", error_message);
                return SimpleResponse {
                    error: Some(error_message),
                    ..response
                };
            }

            attempt += 1;
            let backoff_ms = 2_u64.pow(attempt) * 100;

            log_w!(
                "Network error ({}), will retry after {}ms...\n{}",
                attempt,
                backoff_ms,
                error_message
            );

            std::thread::sleep(Duration::from_millis(backoff_ms));
        }
    }

    fn execute_single_request(
        &self,
        method: &HttpMethod,
        request_args: &RequestArgs,
    ) -> SimpleResponse {
        let mut post_body = None;

        if let Some(body) = &request_args.body {
            post_body = match serde_json::to_string(body) {
                Ok(b) => Some(b),
                Err(e) => {
                    return SimpleResponse {
                        status_code: 0,
                        error: Some(e.to_string()),
                        data: None,
                    };
                }
            };
        }

        let mut easy = Easy::new();
        if let Err(err) = self.configure_request(&mut easy, method, post_body, request_args) {
            return SimpleResponse {
                status_code: 0,
                error: Some(err.to_string()),
                data: None,
            };
        }

        log_i!("Begin Request {:?}", easy);

        let mut buffer = Vec::new();

        let start_time = std::time::Instant::now();
        {
            let mut transfer = easy.transfer();
            if let Err(err) = transfer
                .write_function(|data| {
                    buffer.extend_from_slice(data);
                    Ok(data.len())
                }) {
                return SimpleResponse {
                    status_code: 0,
                    error: Some(err.to_string()),
                    data: None,
                };
            }

            if let Err(err) = transfer.perform() {
                return SimpleResponse {
                    status_code: 0,
                    error: Some(err.to_string()),
                    data: None,
                };
            }
        }

        let duration = start_time.elapsed();
        let status = easy.response_code().unwrap_or(0) as u16;
        log_i!(
            "Response ({}), took {}ms: {}",
            status,
            duration.as_millis(),
            request_args.url
        );

        let data = String::from_utf8(buffer).ok();

        SimpleResponse {
            status_code: status,
            data,
            error: None,
        }
    }

    fn configure_request(
        &self,
        easy: &mut Easy,
        method: &HttpMethod,
        post_body: Option<String>,
        request_args: &RequestArgs,
    ) -> Result<(), curl::Error> {
        easy.url(&request_args.url)?;
        easy.timeout(Duration::from_secs(10))?;

        let mut headers = List::new();
        for (key, value) in &self.headers {
            headers.append(&format!("{}: {}", key, value))?;
        }

        if *method == HttpMethod::POST {
            easy.post(true)?;
        }

        if let Some(body) = post_body {
            easy.post_fields_copy(body.as_bytes())?;
            headers.append("Content-Type: application/json")?;
        }

        if let Some(additional_headers) = &request_args.headers {
            for (key, value) in additional_headers {
                headers.append(&format!("{}: {}", key, value))?;
            }
        }
        easy.http_headers(headers)?;

        if let Some(params) = &request_args.query_params {
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            easy.url(&format!("{}?{}", request_args.url, query_string))?;
        }

        Ok(())
    }
}
