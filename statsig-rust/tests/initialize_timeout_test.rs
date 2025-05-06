mod utils;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

use statsig_rust::{Statsig, StatsigOptions, StatsigUser};

const SDK_KEY: &str = "secret-key";
async fn setup() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/eval_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read file");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            delay_ms: 100,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
}

#[tokio::test]
async fn test_initialize_timeout() {
    let user = StatsigUser::with_user_id("my_user".to_string());
    let scrapi = setup().await;
    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            specs_url: Some(scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            environment: Some("development".to_string()),
            init_timeout_ms: Some(1),
            specs_sync_interval_ms: Some(1),
            output_log_level: Some(statsig_rust::output_logger::LogLevel::Debug),
            ..StatsigOptions::new()
        })),
    );

    let res = statsig.initialize().await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Initialization timed out"));

    let result = statsig.get_feature_gate(&user, "public_dev_only");

    assert!(result.details.reason == "Loading:Unrecognized");
    assert_eventually!(|| scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 2);
    // assert background syncs started
}
