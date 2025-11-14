mod utils;

use parking_lot::Mutex;
use statsig_rust::{output_logger::LogLevel, Statsig, StatsigOptions};
use std::{sync::Arc, time::Duration};
use utils::mock_scrapi::{self, Endpoint, EndpointStub, MockScrapi};

use crate::utils::{
    helpers::load_contents,
    mock_log_provider::{MockLogProvider, RecordedLog},
};

macro_rules! assert_log_contains {
    ($logs:expr, $log_type:ident, $pattern:expr) => {
        assert!($logs.iter().any(|log| match log {
            RecordedLog::$log_type(_, msg) => msg.contains($pattern),
            _ => false,
        }));
    };
}

#[tokio::test]
async fn test_initial_success_with_background_failure() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let dcs = load_contents("eval_proj_dcs.json");

    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            endpoint: Endpoint::DownloadConfigSpecs,
            response: dcs,
            status: 200,
            method: mock_scrapi::Method::GET,
            delay_ms: 0,
        })
        .await;

    let options = StatsigOptions {
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        specs_sync_interval_ms: Some(10),
        output_log_level: Some(LogLevel::Debug),
        output_logger_provider: Some(provider.clone()),
        ..StatsigOptions::default()
    };

    println!(
        "mock scrapi url: {}",
        mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)
    );

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    let init_result = statsig.initialize().await;

    assert!(init_result.is_ok());

    mock_scrapi.clear_stubs().await;

    println!(
        "resetting mock scrapi {}",
        mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)
    );

    mock_scrapi
        .stub(EndpointStub {
            endpoint: Endpoint::DownloadConfigSpecs,
            response: "".to_string(),
            status: 500,
            method: mock_scrapi::Method::GET,
            delay_ms: 0,
        })
        .await;

    assert_eventually!(|| {
        let logs = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
        logs.iter().any(|log| match log {
            RecordedLog::Warn(_, msg) => msg.contains("RetriesExhausted"),
            _ => false,
        })
    });

    let logs = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
    assert_log_contains!(logs, Info, "Network request failed with status code");
    assert_log_contains!(logs, Info, "attempt 1/4");
    assert_log_contains!(logs, Info, "attempt 2/4");
    assert_log_contains!(logs, Info, "attempt 3/4");
    assert_log_contains!(logs, Warn, "status(500) attempts(4) Internal Server Error");
}
