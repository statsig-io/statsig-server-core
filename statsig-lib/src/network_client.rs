use crate::{log_e, log_i};
use reqwest::header::HeaderName;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct NetworkClient {
    client: Client,
    headers: HeaderMap,
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

impl NetworkClient {
    pub fn new(headers: Option<HashMap<String, String>>) -> Self {
        let headers = headers.unwrap_or_default();
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(h_key), Ok(h_value)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                header_map.insert(h_key, h_value);
            }
        }

        NetworkClient {
            client: Client::new(),
            headers: header_map,
            retry_codes: HashSet::from([408, 500, 502, 503, 504, 522, 524, 599]),
        }
    }

    pub async fn get(&self, request_args: RequestArgs) -> Option<String> {
        self.execute_request(Method::GET, &request_args).await
    }

    pub async fn post(&self, request_args: RequestArgs) -> Option<String> {
        self.execute_request(Method::POST, &request_args).await
    }

    async fn execute_request(&self, method: Method, request_args: &RequestArgs) -> Option<String> {
        let mut attempt = 0;

        loop {
            let request = self.build_request(method.clone(), request_args);
            log_i!(
                "Begin Request {} {:?} {:?} {:?} {:?}",
                request_args.url,
                request_args.headers,
                self.headers,
                request_args.query_params,
                json!(request_args.body)
            );

            match request.send().await {
                Ok(response) => {
                    log_i!(
                        "Request Ok ({}): {}",
                        response.status().as_u16(),
                        request_args.url
                    );

                    if response.status().is_success() {
                        return response.text().await.ok();
                    }

                    if !self.retry_codes.contains(&response.status().as_u16()) {
                        return None;
                    }
                }
                Err(e) => {
                    log_e!("Request Err - {}", e);
                }
            }

            if attempt >= request_args.retries {
                return None;
            }

            let backoff_ms = 2_u64.pow(attempt) * 100;
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            attempt += 1;
        }
    }

    fn build_request(&self, method: Method, request_args: &RequestArgs) -> reqwest::RequestBuilder {
        let mut request = self.client.request(method, &request_args.url);
        request = request.timeout(Duration::from_secs(10));

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        if let Some(additional_headers) = &request_args.headers {
            for (key, value) in additional_headers {
                request = request.header(key, value);
            }
        }

        if let Some(params) = &request_args.query_params {
            request = request.query(params);
        }

        if let Some(b) = &request_args.body {
            request = request.json(b);
        }

        request
    }
}
