mod utils;

use futures::future::join_all;
use lazy_static::lazy_static;
use serde_json::{from_str, json, Value};
use statsig_rust::log_event_payload::{LogEventPayload, LogEventRequest};
use statsig_rust::{EventLoggingAdapter, StatsigLocalFileEventLoggingAdapter};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use utils::helpers::assert_eventually;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "server-local-file-events-test";
const EVENTS_FILE_NAME: &str = "3776104929_events.json"; // djb2(SDK_KEY)_events.json

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
    static ref TEST_EVENTS_DATA: Value = from_str(&format!("[{SINGLE_EVENT_DATA}]")).unwrap();
}

async fn setup(test_name: &str) -> (MockScrapi, String) {
    let test_path = format!("/tmp/{}", test_name);

    if std::path::Path::new(&test_path).exists() {
        fs::remove_dir_all(&test_path).unwrap();
    }
    fs::create_dir_all(&test_path).unwrap();

    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    (mock_scrapi, test_path)
}

#[tokio::test]
async fn test_writing_to_file() {
    let (mock_scrapi, tmp_path) = setup("test_writing_to_file").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);
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

    let out_path = format!("{}/{}", tmp_path, EVENTS_FILE_NAME);
    assert_eventually(
        || std::path::Path::new(&out_path).exists(),
        Duration::from_secs(1),
    )
    .await;
}

#[cfg(not(feature = "with_zstd"))]
#[tokio::test]
async fn test_combining_requests() {
    use std::io::Read;

    let (mock_scrapi, tmp_path) = setup("test_combining_requests").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);

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
    let mut decoder = flate2::read::GzDecoder::new(zipped_body.as_slice());
    let mut decompressed_body = String::new();
    decoder.read_to_string(&mut decompressed_body).unwrap();

    let parsed_body: LogEventPayload = from_str(&decompressed_body).unwrap();
    assert_eq!(parsed_body.events.as_array().unwrap().len(), 2);
}

#[cfg(feature = "with_zstd")]
#[tokio::test]
async fn test_combining_requests() {
    let (mock_scrapi, tmp_path) = setup("test_combining_requests").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);

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

    let zstd_body = &requests[0].body;
    let decompressed_body =
        String::from_utf8(zstd::decode_all(zstd_body.as_slice()).unwrap()).unwrap();

    let parsed_body: LogEventPayload = from_str(&decompressed_body).unwrap();
    assert_eq!(parsed_body.events.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_combining_limits() {
    let (mock_scrapi, tmp_path) = setup("test_combining_limits").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);

    for _ in 0..1000 {
        let payload = LogEventPayload {
            events: from_str(&format!("[{SINGLE_EVENT_DATA}, {SINGLE_EVENT_DATA}]")).unwrap(),
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
async fn test_sending_events_over_network() {
    let (mock_scrapi, tmp_path) = setup("test_sending_events_over_network").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);

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

    let out_path = format!("{}/{}", tmp_path, EVENTS_FILE_NAME);
    assert_eventually(
        || fs::read_to_string(&out_path).unwrap().is_empty(),
        Duration::from_secs(1),
    )
    .await;
}

#[tokio::test]
async fn test_exposure_dedupe() {
    let (mock_scrapi, tmp_path) = setup("test_exposure_dedupe").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);
    let adapter = StatsigLocalFileEventLoggingAdapter::new(SDK_KEY, &tmp_path, Some(url), false);

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
async fn test_concurrent_usage() {
    let (mock_scrapi, tmp_path) = setup("test_concurrent_usage").await;

    let url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);
    let adapter = Arc::new(StatsigLocalFileEventLoggingAdapter::new(
        SDK_KEY,
        &tmp_path,
        Some(url),
        false,
    ));

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
