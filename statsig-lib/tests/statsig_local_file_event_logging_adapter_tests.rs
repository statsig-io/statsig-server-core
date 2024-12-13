mod utils;

use lazy_static::lazy_static;
use serde_json::{from_str, json, Value};
use sigstat::log_event_payload::{LogEventPayload, LogEventRequest};
use sigstat::{EventLoggingAdapter, StatsigLocalFileEventLoggingAdapter};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "server-local-file-events-test";
const OUTPUT_FILE_PATH: &str = "/tmp/3776104929_events.json"; // djb2(SDK_KEY)_events.json

const EVENT_DATA: &str = r#"{
    "eventName": "my_custom_event"
}"#;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn get_test_lock() -> MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap();

    if PathBuf::from(OUTPUT_FILE_PATH).exists() {
        fs::remove_file(OUTPUT_FILE_PATH).expect("Failed to delete the events file.");
    }

    guard
}

async fn setup() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    mock_scrapi
}

fn get_requests_data() -> Vec<Value> {
    let file_contents =
        from_str::<Value>(&std::fs::read_to_string(OUTPUT_FILE_PATH).unwrap()).unwrap();

    file_contents
        .get("requests")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.to_owned())
        .collect()
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_writing_to_file() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));
    adapter
        .log_events(LogEventRequest {
            payload: LogEventPayload {
                events: json!(EVENT_DATA),
                statsig_metadata: json!("{}"),
            },
            event_count: 1,
        })
        .await
        .unwrap();

    assert!(
        std::path::Path::new(OUTPUT_FILE_PATH).exists(),
        "The events file was not created."
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_combining_requests() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));

    let payload = LogEventPayload {
        events: json!(EVENT_DATA),
        statsig_metadata: json!("{}"),
    };

    adapter
        .log_events(LogEventRequest {
            payload: payload.clone(),
            event_count: 1,
        })
        .await
        .unwrap();

    adapter
        .log_events(LogEventRequest {
            payload: payload.clone(),
            event_count: 1,
        })
        .await
        .unwrap();

    let requests = get_requests_data();

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].get("eventCount").unwrap(), 2);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_combining_limits() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));

    let event_data = r#"{
        "eventName": "my_custom_event"
    }"#;

    let payload = LogEventPayload {
        events: json!(event_data),
        statsig_metadata: json!("{}"),
    };

    for _ in 0..1000 {
        adapter
            .log_events(LogEventRequest {
                payload: payload.clone(),
                event_count: 1,
            })
            .await
            .unwrap();
    }

    let requests = get_requests_data();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].get("eventCount").unwrap(), 500);
    assert_eq!(requests[1].get("eventCount").unwrap(), 500);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sending_events_over_network() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));

    adapter
        .log_events(LogEventRequest {
            payload: LogEventPayload {
                events: json!(EVENT_DATA),
                statsig_metadata: json!("{}"),
            },
            event_count: 1,
        })
        .await
        .unwrap();

    adapter.send_pending_events().await.unwrap();

    assert!(
        !std::path::Path::new(&OUTPUT_FILE_PATH).exists(),
        "The file should not exist."
    );
}
