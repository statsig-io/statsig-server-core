mod utils;

use sigstat::{output_logger::LogLevel, Statsig, StatsigOptions, StatsigUser};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};
use crate::utils::mock_specs_adapter::MockSpecsAdapter;

const SDK_KEY: &str = "secret-key";

async fn setup(options: StatsigOptions) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
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
        SDK_KEY,
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
    let (scrapi, statsig) = setup(StatsigOptions {
        event_logging_flush_interval_ms: Some(10),
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/eval_proj_dcs.json",
        ))),
        ..StatsigOptions::new()
    }).await;

    statsig.initialize().await.unwrap();

    let user = StatsigUser::with_user_id("a_user".to_string());
    statsig.log_event(&user, "my_event", None, None);

    sleep(Duration::from_millis(100)).await;

    let times_called = scrapi.times_called_for_endpoint(Endpoint::LogEvent);
    assert_eq!(1, times_called);
}

