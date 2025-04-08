mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{output_logger::LogLevel, Statsig, StatsigOptions, StatsigUser};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use utils::{
    helpers::assert_eventually,
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
};

async fn setup(delay_ms: u64, options: StatsigOptions, key: String) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            delay_ms,
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/eval_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read file");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let statsig = Statsig::new(
        &key,
        Some(Arc::new(StatsigOptions {
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            output_log_level: Some(LogLevel::Debug),
            ..options
        })),
    );

    (mock_scrapi, statsig)
}

#[tokio::test]
async fn test_background_flushing() {
    let (scrapi, statsig) = setup(
        0,
        StatsigOptions {
            event_logging_flush_interval_ms: Some(1),
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            ..StatsigOptions::new()
        },
        "key-1".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    let user = StatsigUser::with_user_id("a_user".to_string());
    statsig.log_event(&user, "my_event", None, None);

    assert_eventually(
        || scrapi.times_called_for_endpoint(Endpoint::LogEvent) > 0,
        Duration::from_secs(1),
    )
    .await;

    let times_called = scrapi.times_called_for_endpoint(Endpoint::LogEvent);
    assert_eq!(1, times_called);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn flush_limit_batching_awaiting() {
    std::env::set_var("BATCHING_INTERVAL", "10");
    let (scrapi, statsig) = setup(
        0,
        StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            event_logging_max_queue_size: Some(10),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        },
        "key-2".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    for i in 0..100 {
        let user = StatsigUser::with_user_id(format!("user_{i}"));
        statsig.log_event(&user, "my_event", None, None);
    }

    // let some batch through
    tokio::time::sleep(Duration::from_millis(20)).await;
    let flushed_logs = scrapi.get_logged_event_count();
    assert!(flushed_logs > 0 && flushed_logs < 100);

    statsig.shutdown().await.unwrap();

    let flushed_logs = scrapi.get_logged_event_count();
    assert_eq!(100, flushed_logs);
    std::env::remove_var("BATCHING_INTERVAL");
}

#[tokio::test]
async fn test_limit_batching_shutdown() {
    let (scrapi, statsig) = setup(
        0,
        StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        },
        "key-2".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    for i in 0..9999 {
        let user = StatsigUser::with_user_id(format!("user_{i}"));
        statsig.log_event(&user, "my_event", None, None);
    }

    sleep(Duration::from_millis(100)).await; // wait for diagnostics

    statsig.shutdown().await.unwrap();

    let requests = scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_eq!(10, requests.len());
    let event_count = scrapi.get_logged_event_count();
    assert_eq!(10000, event_count);
}

#[tokio::test]
async fn test_dropping_events() {
    let (scrapi, statsig) = setup(
        0,
        StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        },
        "key-2".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    for i in 0..10999 {
        // 11 batches
        let user = StatsigUser::with_user_id(format!("user_{i}"));
        statsig.log_event(&user, "my_event", None, None);
    }

    sleep(Duration::from_millis(100)).await; // wait for diagnostics

    statsig.shutdown().await.unwrap();

    let requests = scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_eq!(10, requests.len());
    let event_count = scrapi.get_logged_event_count();
    assert_eq!(10000, event_count);
}

#[tokio::test]
async fn test_batch_when_full_but_no_immediate_flush() {
    let (scrapi, statsig) = setup(
        0,
        StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        },
        "key-2".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    for i in 0..999 {
        let user = StatsigUser::with_user_id(format!("user_{i}"));
        statsig.log_event(&user, "my_event", None, None);
    }

    let requests = scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_eq!(0, requests.len());

    assert_eventually_eq!(
        || scrapi.get_requests_for_endpoint(Endpoint::LogEvent).len(),
        1,
        Duration::from_millis(200)
    )
    .await;

    statsig.shutdown().await.unwrap();
}
