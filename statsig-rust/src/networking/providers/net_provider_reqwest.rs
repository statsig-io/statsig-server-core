use std::collections::HashMap;
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::time::Duration;

use async_trait::async_trait;

use crate::{
    log_e, log_w,
    networking::{
        http_types::{HttpMethod, RequestArgs, Response, ResponseData},
        NetworkProvider,
    },
    StatsigErr,
};

use crate::networking::proxy_config::ProxyConfig;
use reqwest::Method;

const TAG: &str = "NetworkProviderReqwest";

pub struct NetworkProviderReqwest {}

#[async_trait]
impl NetworkProvider for NetworkProviderReqwest {
    async fn send(&self, method: &HttpMethod, args: &RequestArgs) -> Response {
        if let Some(is_shutdown) = &args.is_shutdown {
            if is_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                return Response {
                    status_code: None,
                    data: None,
                    error: Some("Request was shutdown".to_string()),
                    headers: None,
                };
            }
        }

        let request = self.build_request(method, args);

        let mut error = None;
        let mut status_code = None;
        let mut data = None;
        let mut headers = None;

        match request.send().await {
            Ok(response) => {
                status_code = Some(response.status().as_u16());
                headers = get_response_headers(&response);

                match Self::write_response_to_temp_file(response).await {
                    Ok(response_data) => data = Some(response_data),
                    Err(e) => {
                        error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                let error_message = get_error_message(e);
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

        let mut client_builder = reqwest::Client::builder();

        // configure proxy if available
        if let Some(proxy_config) = request_args.proxy_config.as_ref() {
            client_builder = Self::configure_proxy(client_builder, proxy_config);
        }

        let client = client_builder.build().unwrap_or_else(|e| {
            log_e!(TAG, "Failed to build reqwest client with proxy config: {}. Falling back to default client.", e);
            reqwest::Client::new()
        });

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
                Some(b) => b.clone(),
                None => vec![],
            };
            let byte_len = bytes.len();

            request = request.body(bytes);
            request = request.header("Content-Length", byte_len.to_string());
        }

        request
    }

    fn configure_proxy(
        client_builder: reqwest::ClientBuilder,
        proxy_config: &ProxyConfig,
    ) -> reqwest::ClientBuilder {
        let (Some(host), Some(port)) = (&proxy_config.proxy_host, &proxy_config.proxy_port) else {
            return client_builder;
        };

        let proxy_url = format!(
            "{}://{}:{}",
            proxy_config.proxy_protocol.as_deref().unwrap_or("http"),
            host,
            port
        );

        let Ok(proxy) = reqwest::Proxy::all(&proxy_url) else {
            log_w!(TAG, "Failed to create proxy for URL: {}", proxy_url);
            return client_builder;
        };

        let Some(auth) = &proxy_config.proxy_auth else {
            return client_builder.proxy(proxy);
        };

        let Some((username, password)) = auth.split_once(':') else {
            log_w!(
                TAG,
                "Invalid proxy auth format. Expected 'username:password'"
            );
            return client_builder.proxy(proxy);
        };

        client_builder.proxy(proxy.basic_auth(username, password))
    }

    async fn write_response_to_temp_file(
        response: reqwest::Response,
    ) -> Result<ResponseData, StatsigErr> {
        let mut response = response;
        let mut temp_file = tempfile::spooled_tempfile(1024 * 1024 * 2); // 2MB

        while let Some(item) = response
            .chunk()
            .await
            .map_err(|e| StatsigErr::FileError(e.to_string()))?
        {
            temp_file
                .write_all(&item)
                .map_err(|e| StatsigErr::FileError(e.to_string()))?;
        }

        temp_file
            .seek(SeekFrom::Start(0))
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        let reader = BufReader::new(temp_file);
        Ok(ResponseData::from_stream(Box::new(reader)))
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
