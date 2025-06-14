mod utils;

use std::sync::{Arc, Mutex};

use statsig_rust::{
    networking::{NetworkClient, RequestArgs},
    Statsig, StatsigOptions,
};
use utils::{
    mock_log_provider::{MockLogProvider, RecordedLog},
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
};

lazy_static::lazy_static! {
    static ref MOCK_LOG_PROVIDER: Arc<MockLogProvider> = Arc::new(MockLogProvider::new());
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

async fn setup(
    response: &str,
    status: u16,
) -> (Arc<MockLogProvider>, MockScrapi, NetworkClient, RequestArgs) {
    let key = "secret-network_failure_tests";

    let mock_log_provider = MOCK_LOG_PROVIDER.clone();
    mock_log_provider.clear();

    let mock_scrapi = MockScrapi::new().await;
    let network_client = NetworkClient::new(key, None, None);

    let options = Arc::new(StatsigOptions {
        output_logger_provider: Some(mock_log_provider.clone()),
        ..StatsigOptions::new()
    });

    // just to get the output logger initialized
    let _ = Statsig::new(key, Some(options));

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            status,
            response: response.to_string(),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let request_args = RequestArgs {
        url: mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs),
        retries: 2,
        ..RequestArgs::new()
    };

    (mock_log_provider, mock_scrapi, network_client, request_args)
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_non_retryable_error_result() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_, _, network_client, request_args) = setup("read the docs", 400).await;

    let url = request_args.url.clone();
    let response = network_client.post(request_args, None).await;

    let error = response.err().unwrap();
    let expected = format!("RequestNotRetryable: {url} status(400) Bad Request: read the docs",);
    assert_eq!(error.to_string(), expected);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_non_retryable_std_out() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (mock_log_provider, _, network_client, request_args) = setup("read the docs", 400).await;

    let url = request_args.url.clone();
    let _ = network_client.post(request_args, None).await;

    let logs = mock_log_provider.logs.lock().unwrap();

    let expected = format!("RequestNotRetryable: {url} status(400) Bad Request: read the docs",);
    assert!(logs.iter().any(|log| match log {
        RecordedLog::Warn(_, msg) => {
            msg.contains(&expected)
        }
        _ => false,
    }));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_exhaust_retries_error_result() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_, _, network_client, request_args) = setup("{}", 500).await;

    let url = request_args.url.clone();
    let response = network_client.post(request_args, None).await;

    let error = response.err().unwrap();
    // 1 initial + 2 retries = 3 attempts
    let expected =
        format!("RetriesExhausted: {url} status(500) attempts(3) Internal Server Error: {{}}",);
    assert_eq!(error.to_string(), expected);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_exhaust_retries_error_result_binary_response() {
    let _lock = TEST_LOCK.lock().unwrap();
    let data = "����            H	  � �        �  ";
    let (_, _, network_client, request_args) = setup(data, 500).await;

    let url = request_args.url.clone();
    let response = network_client.post(request_args, None).await;

    let error = response.err().unwrap();
    // 1 initial + 2 retries = 3 attempts
    let expected =
        format!("RetriesExhausted: {url} status(500) attempts(3) Internal Server Error",);
    assert_eq!(error.to_string(), expected);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_exhaust_retries_std_out() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (mock_log_provider, _, network_client, request_args) = setup("{}", 500).await;

    let _ = network_client.post(request_args, None).await;

    let logs = mock_log_provider.logs.lock().unwrap();
    assert!(logs.iter().any(|log| match log {
        RecordedLog::Warn(_, msg) => {
            msg.contains("Network request failed with status code 500 (attempt 1)")
        }
        _ => false,
    }));

    let found_second_log = logs.iter().any(|log| match log {
        RecordedLog::Warn(_, msg) => {
            msg.contains("Network request failed with status code 500 (attempt 2)")
        }
        _ => false,
    });
    assert!(found_second_log);
}
