mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{output_logger::LogLevel, Statsig, StatsigOptions, StatsigUser};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "secret-key";

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
            event_logging_flush_interval_ms: Some(10),
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

    sleep(Duration::from_millis(100)).await;
    statsig.shutdown().await.unwrap();

    let times_called = scrapi.times_called_for_endpoint(Endpoint::LogEvent);
    assert_eq!(1, times_called);
}

#[tokio::test]
async fn test_limit_flush_awaiting() {
    let (scrapi, statsig) = setup(
        100,
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

    // let the requests start before resetting the mock
    tokio::time::sleep(Duration::from_millis(10)).await;

    scrapi.reset().await;
    scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            delay_ms: 0,
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let user = StatsigUser::with_user_id("final_user".to_string());
    statsig.log_event(&user, "final_event", None, None);
    statsig.flush_events().await;

    let flushed_logs = scrapi.get_logged_event_count();
    assert_eq!(102, flushed_logs);
}
