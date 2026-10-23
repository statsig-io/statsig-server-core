use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::hashing::hash_one;
use crate::log_d;
use crate::{
    log_e, log_w,
    networking::{
        http_types::{HttpMethod, RequestArgs, Response, ResponseData, CLIENT_CONFIG_ERROR_PREFIX},
        NetworkProvider,
    },
    utils::url_path_has_suffix,
    StatsigErr,
};

use crate::networking::proxy_config::ProxyConfig;
use reqwest::Method;

const TAG: &str = "NetworkProviderReqwest";
const LOG_EVENT_REUSE_PATH: &[&str] = &["v1", "log_event"];

// Key material a cached client was built from. Stored alongside the client
// so a u64 hash collision is caught by an exact equality check instead of
// silently serving a client built for a different proxy/trust config.
struct CachedClient {
    proxy_config: Option<ProxyConfig>,
    ca_cert_pem: Option<Vec<u8>>,
    ssl_cert_file: Option<OsString>,
    ssl_cert_dir: Option<OsString>,
    disable_reuse: bool,
    disable_system_proxy: bool,
    client: reqwest::Client,
}

pub struct NetworkProviderReqwest {
    has_file_write_access: bool,
    // Unbounded, but bounded in practice by the number of distinct
    // (proxy_config, ca_cert_pem, SSL_CERT_FILE, SSL_CERT_DIR) combinations
    // seen by this process. Entries are never evicted since each holds a
    // reusable connection pool that we want to keep alive for the process
    // lifetime.
    clients: Mutex<HashMap<u64, CachedClient>>,
}

impl NetworkProviderReqwest {
    pub fn new() -> Self {
        Self {
            has_file_write_access: tempfile::tempfile().is_ok(),
            clients: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for NetworkProviderReqwest {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkProvider for NetworkProviderReqwest {
    async fn send(&self, method: &HttpMethod, args: &RequestArgs) -> Response {
        if let Some(is_shutdown) = &args.is_shutdown {
            if is_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                return Response {
                    status_code: None,
                    data: None,
                    error: Some("Request was shutdown".to_string()),
                };
            }
        }

        let request = match self.build_request(method, args) {
            Ok(request) => request,
            Err(message) => {
                log_e!(TAG, "{}", message);
                return Response {
                    status_code: None,
                    data: None,
                    error: Some(format!("{CLIENT_CONFIG_ERROR_PREFIX}{message}")),
                };
            }
        };

        let mut error = None;
        let mut status_code = None;
        let mut data = None;

        match request.send().await {
            Ok(response) => {
                status_code = Some(response.status().as_u16());

                let data_result =
                    if !self.has_file_write_access || args.disable_file_streaming == Some(true) {
                        Self::write_response_to_in_memory_buffer(response).await
                    } else {
                        Self::write_response_to_temp_file(response).await
                    };

                match data_result {
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
        }
    }

    fn warm(&self, args: &RequestArgs) {
        if let Err(message) = self.get_client(args) {
            log_d!(TAG, "Failed to warm HTTP client: {}", message);
        }
    }
}

impl NetworkProviderReqwest {
    fn build_request(
        &self,
        method: &HttpMethod,
        request_args: &RequestArgs,
    ) -> Result<reqwest::RequestBuilder, String> {
        let method_actual = match method {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
        };
        let is_post = method_actual == Method::POST;

        let client = self.get_client(request_args)?;

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

        Ok(request)
    }

    fn get_client(&self, request_args: &RequestArgs) -> Result<reqwest::Client, String> {
        // Contract: without the reuse flag, log_event must not reuse connections
        // across requests. We still cache the *client* under a distinct key
        // (built with pool_max_idle_per_host(0), so it never keeps an idle
        // connection around and every request opens a fresh one) because with
        // rustls-tls-native-roots, building a client reloads the OS trust
        // store, which is expensive (>100ms on macOS) and would otherwise
        // happen on every single log_event flush.
        let disable_reuse =
            is_log_event_endpoint(&request_args.url) && !request_args.log_event_connection_reuse;

        let key = client_cache_key(request_args, disable_reuse);
        let ssl_cert_file = std::env::var_os("SSL_CERT_FILE");
        let ssl_cert_dir = std::env::var_os("SSL_CERT_DIR");

        // No lock is held across the (potentially slow) build below.
        if let Some(cached) = self.clients.lock().get(&key) {
            if cached.proxy_config == request_args.proxy_config
                && cached.ca_cert_pem == request_args.ca_cert_pem
                && cached.ssl_cert_file == ssl_cert_file
                && cached.ssl_cert_dir == ssl_cert_dir
                && cached.disable_reuse == disable_reuse
                && cached.disable_system_proxy == request_args.disable_system_proxy
            {
                return Ok(cached.client.clone());
            }
            log_w!(
                TAG,
                "Network client cache key collision detected; rebuilding client"
            );
        }

        let client = Self::build_client(request_args, disable_reuse)?;
        self.clients.lock().insert(
            key,
            CachedClient {
                proxy_config: request_args.proxy_config.clone(),
                ca_cert_pem: request_args.ca_cert_pem.clone(),
                ssl_cert_file,
                ssl_cert_dir,
                disable_reuse,
                disable_system_proxy: request_args.disable_system_proxy,
                client: client.clone(),
            },
        );
        Ok(client)
    }

    fn build_client(
        request_args: &RequestArgs,
        disable_connection_reuse: bool,
    ) -> Result<reqwest::Client, String> {
        let mut client_builder = reqwest::Client::builder();
        if request_args.disable_system_proxy {
            client_builder = client_builder.no_proxy();
        }

        if disable_connection_reuse {
            client_builder = client_builder.pool_max_idle_per_host(0);
        }

        // configure proxy if available
        if let Some(proxy_config) = request_args.proxy_config.as_ref() {
            client_builder = Self::configure_proxy(client_builder, proxy_config);
        }

        if let Some(ca_cert_pem) = &request_args.ca_cert_pem {
            let parsed = crate::networking::ca_bundle::parse_ca_certs(ca_cert_pem);
            if parsed.certs.is_empty() {
                let detail = if parsed.skipped > 0 {
                    format!("{} invalid entries skipped", parsed.skipped)
                } else {
                    "file contains no PEM certificates".to_string()
                };
                return Err(format!(
                    "No usable certificates found in proxy_config.ca_cert_path ({detail}). \
                     Provide a PEM file or bundle containing CA certificates."
                ));
            }
            if parsed.skipped > 0 {
                log_w!(
                    TAG,
                    "Loaded {} certificate(s) from proxy_config.ca_cert_path; skipped {} unparseable entries",
                    parsed.certs.len(),
                    parsed.skipped
                );
            }
            for cert in parsed.certs {
                client_builder = client_builder.add_root_certificate(cert);
            }
        }

        client_builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))
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
        let headers = get_response_headers(&response);
        let mut response = response;
        let mut temp_file = tempfile::spooled_tempfile(1024 * 1024 * 2); // 2MB

        let mut total_bytes = 0;
        while let Some(item) = response
            .chunk()
            .await
            .map_err(|e| StatsigErr::FileError(e.to_string()))?
        {
            total_bytes += item.len();
            temp_file
                .write_all(&item)
                .map_err(|e| StatsigErr::FileError(e.to_string()))?;
        }

        temp_file
            .seek(SeekFrom::Start(0))
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        let reader = BufReader::new(temp_file);

        log_d!(TAG, "Wrote {} bytes to spooled temp file", total_bytes);

        Ok(ResponseData::from_stream_with_headers(
            Box::new(reader),
            headers,
        ))
    }

    async fn write_response_to_in_memory_buffer(
        response: reqwest::Response,
    ) -> Result<ResponseData, StatsigErr> {
        let headers = get_response_headers(&response);
        let bytes = response
            .bytes()
            .await
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))?;

        log_d!(TAG, "Wrote {} bytes to in-memory buffer", bytes.len());

        Ok(ResponseData::from_bytes_with_headers(
            bytes.to_vec(),
            headers,
        ))
    }
}

const TLS_HINT: &str = " Hint: TLS certificate verification failed. If this host is behind a \
TLS-inspecting proxy (e.g. Zscaler, Netskope), install the proxy root CA into the OS trust \
store, set SSL_CERT_FILE, or set StatsigOptions.proxy_config.ca_cert_path to a PEM bundle.";

fn get_error_message(error: reqwest::Error) -> String {
    let mut error_message = error.to_string();

    let mut source = std::error::Error::source(&error);
    while let Some(err) = source {
        let cause = err.to_string();
        if !error_message.contains(&cause) {
            error_message.push_str(&format!(": {cause}"));
        }
        source = err.source();
    }

    if let Some(url_error) = error.url() {
        error_message.push_str(&format!(". URL: {}", url_error));
    }

    if let Some(status_error) = error.status() {
        error_message.push_str(&format!(". Status: {}", status_error));
    }

    if error_message.to_ascii_lowercase().contains("certificate") {
        error_message.push_str(TLS_HINT);
    }

    error_message
}

fn is_log_event_endpoint(url: &str) -> bool {
    url_path_has_suffix(url, LOG_EVENT_REUSE_PATH)
}

// Note: SSL_CERT_FILE/SSL_CERT_DIR are hashed here, then read again in
// get_client for the stored-vs-current equality check and once more by
// rustls-native-certs when the client is actually built. Mutating these env
// vars mid-flight is not a supported pattern; if it happens anyway, the
// equality check in get_client catches the mismatch and rebuilds rather than
// silently serving a stale client.
fn client_cache_key(request_args: &RequestArgs, disable_reuse: bool) -> u64 {
    hash_one((
        &request_args.proxy_config,
        &request_args.ca_cert_pem,
        std::env::var_os("SSL_CERT_FILE"),
        std::env::var_os("SSL_CERT_DIR"),
        disable_reuse,
        request_args.disable_system_proxy,
    ))
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

#[cfg(test)]
mod tests {
    use super::is_log_event_endpoint;
    use super::NetworkProviderReqwest;
    use crate::networking::proxy_config::ProxyConfig;
    use crate::networking::RequestArgs;

    #[test]
    fn test_is_log_event_endpoint_matches_exact_suffix() {
        assert!(is_log_event_endpoint(
            "https://api.statsig.com/v1/log_event"
        ));
        assert!(is_log_event_endpoint(
            "https://api.statsig.com/v1/log_event/"
        ));
        assert!(is_log_event_endpoint(
            "https://api.statsig.com/prefix/v1/log_event?foo=bar"
        ));

        assert!(!is_log_event_endpoint(
            "https://api.statsig.com/v1/log_event/extra"
        ));
        assert!(!is_log_event_endpoint(
            "https://api.statsig.com/v1/log_events"
        ));
        assert!(!is_log_event_endpoint("https://api.statsig.com/log_event"));
    }

    #[test]
    fn test_clients_are_cached_per_config() {
        let provider = NetworkProviderReqwest::new();
        let args = RequestArgs {
            url: "https://api.statsigcdn.com/v2/download_config_specs/key.json".into(),
            ..RequestArgs::new()
        };
        let _ = provider.get_client(&args).unwrap();
        let _ = provider.get_client(&args).unwrap();
        assert_eq!(provider.clients.lock().len(), 1);

        let proxied = RequestArgs {
            url: args.url.clone(),
            proxy_config: Some(ProxyConfig {
                proxy_host: Some("proxy.example.com".into()),
                proxy_port: Some(8080),
                proxy_auth: None,
                proxy_protocol: None,
                ca_cert_path: None,
            }),
            ..RequestArgs::new()
        };
        let _ = provider.get_client(&proxied).unwrap();
        assert_eq!(provider.clients.lock().len(), 2);
    }

    #[test]
    fn test_log_event_reuse_flag_selects_distinct_cached_clients() {
        let provider = NetworkProviderReqwest::new();
        let args = RequestArgs {
            url: "https://api.statsig.com/v1/log_event".into(),
            ..RequestArgs::new()
        };
        let _ = provider.get_client(&args).unwrap();
        assert_eq!(provider.clients.lock().len(), 1);

        // Calling again with the same (no-reuse) config must hit the cache,
        // not rebuild the client.
        let _ = provider.get_client(&args).unwrap();
        assert_eq!(provider.clients.lock().len(), 1);

        // Same config but with the reuse flag on must be cached under a
        // distinct key (normal pooling), not collapse into the same entry.
        let reuse = RequestArgs {
            url: args.url.clone(),
            log_event_connection_reuse: true,
            ..RequestArgs::new()
        };
        let _ = provider.get_client(&reuse).unwrap();
        assert_eq!(provider.clients.lock().len(), 2);
    }

    #[tokio::test]
    async fn test_error_message_includes_source_chain() {
        // port 9 (discard) is closed; connect error carries a source chain
        let err = reqwest::Client::new()
            .get("http://127.0.0.1:9")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect_err("must fail");
        let message = super::get_error_message(err);
        assert!(
            message.contains("refused") || message.contains("connect error"),
            "source chain missing from: {message}"
        );
    }
}
