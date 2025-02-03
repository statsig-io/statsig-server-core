use crate::networking::RequestArgs;
use crate::observability::util::sanitize_url_for_logging;
use crate::{log_d, log_e, ok_or_return_with, unwrap_or_return_with, StatsigErr};
use chrono::Utc;
use curl::easy::{Easy2, Handler, List, WriteError};
use curl::multi::Easy2Handle;
use curl::multi::{self, Multi};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::{runtime, time};

use super::{HttpMethod, Response};

const MAX_QUEUED_REQUESTS: usize = 10;

lazy_static::lazy_static! {
    static ref CURL: Mutex<HashMap<String, Arc<CurlContext>>> = Mutex::new(HashMap::new());
}

struct Request {
    method: HttpMethod,
    args: RequestArgs,
    tx: oneshot::Sender<Result<Response, StatsigErr>>,
}

struct ActiveRequest {
    request: Request,
    handle: Easy2Handle<Collector>,
}

struct CurlContext {
    req_tx: mpsc::Sender<Request>,
    _abort_tx: Option<oneshot::Sender<()>>,
    _handle: Option<Arc<JoinHandle<()>>>,
}

const TAG: &str = stringify!(Curl);

pub struct Curl {
    sdk_key: String,
    context: Arc<CurlContext>,
}

impl Drop for Curl {
    fn drop(&mut self) {
        let count = Arc::strong_count(&self.context);

        if count <= 2 {
            if let Ok(mut curl_map) = CURL.lock() {
                curl_map.remove(&self.sdk_key);
            }
        }
    }
}

impl Curl {
    pub fn get(sdk_key: &str) -> Curl {
        let mut curl_map = match CURL.lock() {
            Ok(map) => map,
            Err(e) => {
                log_e!(TAG, "Failed to acquire lock on CURL: {}", e);
                return Curl::new(sdk_key);
            }
        };

        match curl_map.get(sdk_key) {
            Some(curl) => Curl {
                sdk_key: sdk_key.to_string(),
                context: curl.clone(),
            },
            None => {
                let curl = Curl::new(sdk_key);
                curl_map.insert(sdk_key.to_string(), curl.context.clone());
                curl
            }
        }
    }

    pub async fn send(&self, method: &HttpMethod, request_args: &RequestArgs) -> Response {
        let method_name = if method == &HttpMethod::POST {
            "POST"
        } else {
            "GET"
        };
        log_d!(TAG, "Sending {} Request: {}", method_name, request_args.url);

        if let Some(headers) = &request_args.headers {
            for (key, value) in headers.iter() {
                log_d!(TAG, "Header: {} = {}", key, value);
            }
        }

        let (response_tx, response_rx) = oneshot::channel();
        let request = Request {
            method: method.clone(),
            args: request_args.clone(),
            tx: response_tx,
        };

        match self.context.req_tx.send(request).await {
            Ok(_) => (),
            Err(e) => {
                return Response {
                    status_code: 0,
                    data: None,
                    error: Some(e.to_string()),
                }
            }
        }

        let result = response_rx.await.unwrap_or_else(|e| {
            log_e!(TAG, "Failed to receive response: {:?}", e);
            Err(StatsigErr::NetworkError(e.to_string()))
        });

        result.unwrap_or_else(|e| Response {
            status_code: 0,
            data: None,
            error: Some(e.to_string()),
        })
    }

    fn new(sdk_key: &str) -> Curl {
        let (handle, abort_tx, req_tx) = Self::create_run_loop();

        Curl {
            sdk_key: sdk_key.to_string(),
            context: Arc::new(CurlContext {
                req_tx,
                _abort_tx: Some(abort_tx),
                _handle: handle.map(Arc::new),
            }),
        }
    }

    fn create_run_loop() -> (
        Option<JoinHandle<()>>,
        oneshot::Sender<()>,
        mpsc::Sender<Request>,
    ) {
        let (abort_tx, abort_rx) = oneshot::channel::<()>();
        let (req_tx, req_rx) = mpsc::channel::<Request>(MAX_QUEUED_REQUESTS);

        let handle_result = thread::Builder::new()
            .name("curl-run-loop".to_string())
            .spawn(move || {
                let rt = match runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        log_e!(TAG, "Failed to build cURL runtime: {:?}", e);
                        return;
                    }
                };

                rt.block_on(Self::run(abort_rx, req_rx));
            });

        let handle = match handle_result {
            Ok(handle) => {
                log_d!(TAG, "New cURL run loop created.");
                Some(handle)
            }
            Err(e) => {
                log_e!(TAG, "Failed to spawn cURL run loop: {:?}", e);
                None
            }
        };

        (handle, abort_tx, req_tx)
    }

    async fn run(mut abort_rx: oneshot::Receiver<()>, mut req_rx: mpsc::Receiver<Request>) {
        let multi = Multi::new();
        let mut active_reqs = HashMap::new();
        let mut next_token = 0;

        loop {
            tokio::select! {
                _ = &mut abort_rx => {
                    break;
                }
                _ = time::sleep(Duration::from_millis(1)), if !active_reqs.is_empty() => {}
                Some(request) = req_rx.recv() => {
                    if active_reqs.is_empty() {
                        next_token = 0;
                    }

                    if let Err(e) = Self::add_request_for_processing(&multi, &mut active_reqs, &mut next_token, request) {
                        log_e!(TAG, "Failed to add request for processing: {:?}", e);
                    }
                }
            }

            Self::remove_shutdown_requests(&multi, &mut active_reqs);
            Self::process_active_requests(&multi, &mut active_reqs);
        }
    }

    fn add_request_for_processing(
        multi: &Multi,
        handles: &mut HashMap<usize, ActiveRequest>,
        next_token: &mut usize,
        request: Request,
    ) -> Result<(), StatsigErr> {
        let args = &request.args;
        let easy = construct_easy_request(&request.method, args)
            .map_err(|e| StatsigErr::NetworkError(e.to_string()))?;

        match multi.add2(easy) {
            Ok(mut handle) => {
                handle
                    .set_token(*next_token)
                    .map_err(|e| StatsigErr::NetworkError(e.to_string()))?;
                handles.insert(*next_token, ActiveRequest { handle, request });
                *next_token = next_token.wrapping_add(1);
                Ok(())
            }
            Err(e) => Err(StatsigErr::NetworkError(e.to_string())),
        }
    }

    fn remove_shutdown_requests(multi: &multi::Multi, active: &mut HashMap<usize, ActiveRequest>) {
        let to_remove: Vec<usize> = active
            .iter()
            .filter_map(|(token, entry)| {
                if let Some(is_shutdown) = &entry.request.args.is_shutdown {
                    if is_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                        return Some(*token);
                    }
                }
                None
            })
            .collect();

        for token in to_remove {
            if let Some(entry) = active.remove(&token) {
                let _ = entry.request.tx.send(Err(StatsigErr::NetworkError(
                    "Request was shutdown".to_string(),
                )));

                if let Err(e) = multi.remove2(entry.handle) {
                    log_e!(TAG, "Failed to remove request from multi: {:?}", e);
                }
            }
        }
    }

    fn process_active_requests(multi: &Multi, active: &mut HashMap<usize, ActiveRequest>) {
        let perform = match multi.perform() {
            Ok(perform) => perform,
            Err(e) => {
                log_e!(TAG, "Failed to perform requests: {:?}", e);
                return;
            }
        };

        if perform == 0 {
            log_d!(TAG, "No requests performed");
        }

        multi.messages(|msg| {
            let token = ok_or_return_with!(msg.token(), |e| {
                log_e!(TAG, "Failed to get token: {:?}", e);
            });

            let mut entry = unwrap_or_return_with!(active.remove(&token), || {
                log_e!(TAG, "Token not found: {}", token);
            });
            let url = &entry.request.args.url;

            let result = unwrap_or_return_with!(msg.result_for2(&entry.handle), || {
                log_e!(TAG, "Failed to get result for token: {}", token);
            });
            let sanitized_url = sanitize_url_for_logging(url);

            match result {
                Ok(()) => {
                    let http_status = entry.handle.response_code().unwrap_or_else(|e| {
                        log_e!(TAG, "Failed to get HTTP status: {:?}", e);
                        0
                    });

                    let res_buffer = entry.handle.get_mut().get_buffer();
                    log_d!(
                        TAG,
                        "Transfer succeeded (Status: {}) (Download length: {}) {}",
                        http_status,
                        &res_buffer.len(),
                        sanitized_url
                    );

                    let data = String::from_utf8(res_buffer)
                        .map_err(|e| {
                            log_e!(
                                TAG,
                                "Failed to convert response to string: {} {:?}",
                                sanitized_url,
                                e
                            );
                            e
                        })
                        .ok();

                    let response = Response {
                        data,
                        status_code: http_status as u16,
                        error: None,
                    };

                    if entry.request.tx.send(Ok(response)).is_err() {
                        log_e!(TAG, "Failed to broadcast response: {}", sanitized_url);
                    }
                }
                Err(e) => {
                    log_e!(TAG, "Failed to send request to {}: {:?}", sanitized_url, e);
                    let _ = entry
                        .request
                        .tx
                        .send(Err(StatsigErr::NetworkError(e.to_string())));
                    return;
                }
            };

            if let Err(e) = multi.remove2(entry.handle) {
                log_e!(TAG, "Failed to remove request from multi: {:?}", e);
            }

            log_d!(TAG, "Request completed: {}", sanitized_url);
        });
    }
}

fn construct_easy_request(
    method: &HttpMethod,
    args: &RequestArgs,
) -> Result<Easy2<Collector>, curl::Error> {
    let mut easy = Easy2::new(Collector::new());

    if args.timeout_ms > 0 {
        easy.timeout(Duration::from_millis(args.timeout_ms))?;
    } else {
        easy.timeout(Duration::from_secs(10))?;
    }

    if args.accept_gzip_response {
        easy.accept_encoding("gzip")?;
    }

    if *method == HttpMethod::POST {
        easy.post(true)?;
    }

    let mut headers = List::new();

    headers.append(&format!(
        "statsig-client-time: {}",
        Utc::now().timestamp_millis()
    ))?;

    if let Some(body) = &args.body {
        easy.post_fields_copy(body)?;
        headers.append("Content-Type: application/json")?;
    }

    if let Some(additional_headers) = &args.headers {
        for (key, value) in additional_headers {
            headers.append(&format!("{}: {}", key, value))?;
        }
    }
    easy.http_headers(headers)?;

    if let Some(params) = &args.query_params {
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        easy.url(&format!("{}?{}", args.url, query_string))?;
    } else {
        easy.url(&args.url)?;
    }

    Ok(easy)
}

struct Collector {
    buffer: Vec<u8>,
}

impl Collector {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn get_buffer(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }
}

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.buffer.extend_from_slice(data);
        Ok(data.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::Statsig;

    use super::*;
    use more_asserts::assert_le;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;
    use tokio::task;
    use wiremock::{
        http::Method as WiremockMethod,
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    fn test_only_one_instance() {
        let key = "key_1";
        let curl_service_1 = Curl::get(key);
        let curl_service_2 = Curl::get(key);

        assert!(Arc::ptr_eq(
            &curl_service_1.context,
            &curl_service_2.context
        ));
    }

    #[test]
    fn test_creating_multiples() {
        let key = "key_2";

        let mut last = 0;
        for _ in 0..10 {
            assert!(CURL.lock().unwrap().get(key).is_none());
            let c = Curl::get(key);
            let now = Arc::as_ptr(&c.context) as usize;

            assert_ne!(now, last);
            last = now;
            assert!(CURL.lock().unwrap().get(key).is_some());
        }

        assert!(CURL.lock().unwrap().get(key).is_none());
    }

    #[test]
    fn test_drop_releases_instance() {
        let key = "key_3";

        let curl_service_1 = Curl::get(key);
        let curl_service_2 = Curl::get(key);
        assert!(CURL.lock().unwrap().get(key).is_some());

        drop(curl_service_1);
        assert!(CURL.lock().unwrap().get(key).is_some());

        drop(curl_service_2);
        assert!(CURL.lock().unwrap().get(key).is_none());
    }

    #[tokio::test]
    async fn test_shutdown_kills_requests() {
        let key = "key_4";

        let server = MockServer::start().await;

        Mock::given(method(WiremockMethod::GET))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"success\": true}")
                    .set_delay(Duration::from_millis(10_000)),
            )
            .mount(&server)
            .await;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let handle = task::spawn(async move {
            let curl = Curl::get(key);
            curl.send(
                &HttpMethod::GET,
                &RequestArgs {
                    is_shutdown: Some(shutdown_clone),
                    url: format!("{}/test", server.uri()),
                    ..RequestArgs::new()
                },
            )
            .await;
        });

        let start = Instant::now();
        shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        handle.await.unwrap();

        assert_le!(start.elapsed().as_millis(), 100);
        time::sleep(Duration::from_millis(100)).await;
        assert!(CURL.lock().unwrap().get(key).is_none());
    }

    #[tokio::test]
    async fn test_statsig_shutdown_kills_thread() {
        let key = "sdk_key_5";
        let statsig = Statsig::new(key, None);

        let _ = statsig.initialize().await;
        assert!(CURL.lock().unwrap().get(key).is_some());

        let _ = statsig.shutdown().await;
        drop(statsig);

        tokio::time::sleep(Duration::from_millis(1)).await;
        assert!(CURL.lock().unwrap().get(key).is_none());
    }

    #[tokio::test]
    async fn test_thread_dies_on_drop() {
        let key = "sdk_key_6";
        let curl = Curl::get(key);
        let handle = curl.context._handle.clone();

        assert!(!handle.as_ref().is_some_and(|h| h.is_finished()));
        drop(curl);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(handle.as_ref().is_some_and(|h| h.is_finished()));
    }
}
