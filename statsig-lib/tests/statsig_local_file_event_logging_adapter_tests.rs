mod utils;

use lazy_static::lazy_static;
use serde_json::{from_str, json, Value};
use sigstat::log_event_payload::{LogEventPayload, LogEventRequest};
use sigstat::{EventLoggingAdapter, StatsigLocalFileEventLoggingAdapter};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use flate2::read::GzDecoder;
use futures::future::join_all;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "server-local-file-events-test";
const OUTPUT_FILE_PATH: &str = "/tmp/3776104929_events.json"; // djb2(SDK_KEY)_events.json

const SINGLE_EVENT_DATA: &str = r#"{
    "eventName":"foo",
    "metadata":{"key":"value"},
    "secondaryExposures":null,
    "time":1734476293616,
    "user":{
        "statsigEnvironment":null,"userID":"a-user"
    },
    "value":"bar"
}"#;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    static ref TEST_EVENTS_DATA: Value = from_str(&format!("[{}]", SINGLE_EVENT_DATA)).unwrap();
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
                events: TEST_EVENTS_DATA.clone(),
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
        events: TEST_EVENTS_DATA.clone(),
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

    adapter.send_pending_events().await.unwrap();

    let requests = mock_scrapi.get_requests_for_endpoint(Endpoint::LogEvent);

    assert_eq!(requests.len(), 1);

    let zipped_body = &requests[0].body;
    let mut decoder = GzDecoder::new(zipped_body.as_slice());
    let mut decompressed_body = String::new();
    decoder.read_to_string(&mut decompressed_body).unwrap();

    let parsed_body: LogEventPayload = from_str(&decompressed_body).unwrap();
    assert_eq!(parsed_body.events.as_array().unwrap().len(), 2);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_combining_limits() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));



    for _ in 0..1000 {
        let payload = LogEventPayload {
            events: from_str(&format!("[{}, {}]", SINGLE_EVENT_DATA, SINGLE_EVENT_DATA)).unwrap(),
            statsig_metadata: json!("{}"),
        };

        adapter
            .log_events(LogEventRequest {
                payload: payload.clone(),
                event_count: 2,
            })
            .await
            .unwrap();
    }

    adapter.send_pending_events().await.unwrap();

    assert_eq!(mock_scrapi.get_logged_event_count(), 2000);

    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_eq!(reqs.len(), 2);
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
                events: TEST_EVENTS_DATA.clone(),
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

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_exposure_dedupe() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);
    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url));

    let expo_data = r#"[{
        "eventName": "statsig::config_exposure",
        "metadata": {
          "config": "another_experiment",
          "configVersion": "11",
          "lcut": "1734392177735",
          "reason": "Bootstrap:Recognized",
          "receivedAt": "1734485550338",
          "ruleID": "4k8adn0DMHSn0tbn0AxNNQ"
        },
        "secondaryExposures": [],
        "time": 1734485550340,
        "user": {
          "statsigEnvironment": { "tier": "production" },
          "userID": "user_id"
        },
        "value": null
    }]"#;

    for _ in 0..3 {
        adapter
            .log_events(LogEventRequest {
                payload: LogEventPayload {
                    events: from_str(expo_data).unwrap(),
                    statsig_metadata: json!("{}"),
                },
                event_count: 1,
            })
            .await
            .unwrap();
    }

    adapter.send_pending_events().await.unwrap();

    assert_eq!(mock_scrapi.get_logged_event_count(), 1);
    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_eq!(reqs.len(), 1);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_concurrent_usage() {
    let _lock = get_test_lock();
    let mock_scrapi = setup().await;

    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);
    let adapter = Arc::new(StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, "/tmp", Some(url)));

    let mut task = vec![];
    for _ in 0..10 {
        let adapter_clone = adapter.clone();
        task.push(tokio::task::spawn(async move {
            adapter_clone
                .log_events(LogEventRequest {
                    payload: LogEventPayload {
                        events: TEST_EVENTS_DATA.clone(),
                        statsig_metadata: json!("{}"),
                    },
                    event_count: 1,
                })
                .await
                .unwrap();
        }));

        let another_clone = adapter.clone();
        task.push(tokio::task::spawn(async move {
            another_clone.send_pending_events().await.unwrap();
        }));
    }

    join_all(task).await;

    assert_eq!(mock_scrapi.get_logged_event_count(), 10);
}
