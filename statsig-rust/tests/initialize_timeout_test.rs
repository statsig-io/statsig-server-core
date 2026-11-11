mod utils;
use std::sync::Arc;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};
use utils::mock_specs_adapter::MockSpecsAdapter;

use statsig_rust::{
    networking::{NetworkClient, RequestArgs},
    Statsig, StatsigErr, StatsigOptions, StatsigUser,
};

const SDK_KEY: &str = "secret-key";

#[tokio::test]
async fn test_initialize_timeout() {
    let user = StatsigUser::with_user_id("my_user".to_string());
    // Isolate the initialization deadline from the HTTP request timeout.
    let specs_adapter = Arc::new(MockSpecsAdapter::delayed(
        "tests/data/eval_proj_dcs.json",
        60_000,
    ));
    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            disable_all_logging: Some(true),
            disable_country_lookup: Some(true),
            environment: Some("development".to_string()),
            init_timeout_ms: Some(10),
            specs_sync_interval_ms: Some(1),
            output_log_level: Some(statsig_rust::output_logger::LogLevel::Debug),
            ..StatsigOptions::new()
        })),
    );

    let err = statsig.initialize().await.unwrap_err();
    assert!(matches!(
        err,
        StatsigErr::InitializationError(reason) if reason == "Initialization timed out"
    ));

    let result = statsig.get_feature_gate(&user, "public_dev_only");

    assert_eq!(result.details.reason, "Loading:Unrecognized");
    assert_eventually!(|| specs_adapter.background_sync_calls() == 1);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_timeout_logic() {
    let mock_scrapi = MockScrapi::new().await;
    let network_client = NetworkClient::new("secret_key", None, None);

    // Test for init request
    let init_request_args = RequestArgs {
        url: mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs),
        timeout_ms: 20,
        ..RequestArgs::new()
    };

    mock_scrapi
        .stub(EndpointStub {
            delay_ms: 200, // 200 ms delay
            method: Method::POST,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let response = network_client.post(init_request_args, None).await;
    // Assert that the request timed out
    assert!(response.is_err());
}
