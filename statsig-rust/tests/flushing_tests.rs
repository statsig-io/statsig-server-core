mod utils;
use serde_json::Value;
use statsig_rust::{
    output_logger::LogLevel, DynamicConfigEvaluationOptions, ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions, Statsig, StatsigOptions, StatsigUser,
};
use std::{
    io::Read,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use utils::{
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
    mock_specs_adapter::MockSpecsAdapter,
};

async fn setup(delay_ms: u64, key: String) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            delay_ms,
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let statsig = Statsig::new(
        &key,
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            ..StatsigOptions::new()
        })),
    );

    (mock_scrapi, statsig)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (_, statsig) = setup(900 /* delay_ms */, "secret-key-flush-1".to_string()).await;

    statsig.initialize().await.unwrap();

    let start = Instant::now();
    for i in 0..10000 {
        let user = StatsigUser::with_user_id(format!("test_user_{i}"));
        let gate = statsig.check_gate(&user, "test_public");
        assert!(gate);
    }

    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000);
    println!("check_gate: {:.2} ms", duration.as_millis());

    let shutdown_start = Instant::now();
    let _ = statsig
        .shutdown_with_timeout(Duration::from_millis(100))
        .await;
    let shutdown_duration = shutdown_start.elapsed();
    println!("shutdown: {:.2} ms", shutdown_duration.as_millis());

    let overall_duration = start.elapsed();
    println!("overall: {:.2} ms", overall_duration.as_millis());
    assert!(overall_duration.as_millis() < 2000);
}

#[tokio::test]
async fn test_no_flushing_on_main() {
    let (_, statsig) = setup(
        10000, /* delay_ms */
        "secret-key-flushing-2".to_string(),
    )
    .await;

    statsig.initialize().await.unwrap();

    let start = Instant::now();
    for i in 0..15000 {
        let user = StatsigUser::with_user_id(format!("test_user_{i}"));
        let gate = statsig.check_gate(&user, "test_public");
        assert!(gate);
    }

    let duration = start.elapsed();
    println!("check_gate: {:.2} ms", duration.as_millis());
    assert!(duration.as_millis() < 1000);

    let _ = statsig
        .shutdown_with_timeout(Duration::from_millis(100))
        .await;

    let overall_duration = start.elapsed();
    println!("overall: {:.2} ms", overall_duration.as_millis());
    assert!(overall_duration.as_millis() < 5000);
}

#[tokio::test]
async fn test_all_events_get_flushed() {
    let (mock_scrapi, statsig) = setup(0 /* delay_ms */, "secret-key-flushing-3".to_string()).await;

    statsig.initialize().await.unwrap();

    for i in 0..5000 {
        let user = StatsigUser::with_user_id(format!("test_user_{i}"));
        let gate = statsig.check_gate(&user, "test_public");
        assert!(gate);
    }

    sleep(Duration::from_millis(100)).await; // give time for diagnostics to flush too
    let start = Instant::now();
    statsig.shutdown().await.unwrap();

    assert_eventually_eq!(|| mock_scrapi.get_logged_event_count(), 5001);

    let duration = start.elapsed();
    println!("shutdown: {:.2} ms", duration.as_millis());
}

#[tokio::test]
async fn test_core_apis_exposure_logging_disabled() {
    let (mock_scrapi, statsig) = setup(0 /* delay_ms */, "secret-key-flushing-4".to_string()).await;

    statsig.initialize().await.unwrap();
    let user = StatsigUser::with_user_id("test_user".into());

    let _ = statsig.check_gate_with_options(
        &user,
        "test_public",
        FeatureGateEvaluationOptions {
            disable_exposure_logging: true,
        },
    );

    let _ = statsig.get_feature_gate_with_options(
        &user,
        "test_public",
        FeatureGateEvaluationOptions {
            disable_exposure_logging: true,
        },
    );
    let _ = statsig.get_experiment_with_options(
        &user,
        "experiment_with_many_params",
        ExperimentEvaluationOptions {
            disable_exposure_logging: true,
            user_persisted_values: None,
        },
    );
    let _ = statsig.get_dynamic_config_with_options(
        &user,
        "dynamic_config_name",
        DynamicConfigEvaluationOptions {
            disable_exposure_logging: true,
        },
    );

    let start = Instant::now();

    tokio::time::sleep(Duration::from_millis(100)).await; // wait for diagnostics observer to enqueue events

    statsig.shutdown().await.unwrap();

    assert_eventually_eq!(
        || mock_scrapi.get_logged_event_count(),
        2 // TODO add the ability to filter non-diagnostic events, 2 = 1 diagnostic + 1 non exposed checks
    );

    let duration = start.elapsed();
    println!(
        "shutdown: {:.2} ms",
        (duration.as_millis() as i128 - 10) as f64
    );
}

#[tokio::test]
async fn test_flushing_backoff_and_metadata() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS", "4");

    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            status: 502,
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            output_log_level: Some(LogLevel::Debug),
            disable_user_agent_parsing: Some(true),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );

    statsig.initialize().await.unwrap();

    let user = StatsigUser::with_user_id("test_user".to_string());
    statsig.log_event(&user, "event_name", None, None);

    assert_eventually!(|| mock_scrapi
        .get_requests_for_endpoint(Endpoint::LogEvent)
        .len()
        > 3);

    let log_event_requests = mock_scrapi.get_requests_for_endpoint(Endpoint::LogEvent);
    assert_flushing_interval_and_scheduled_batch(&log_event_requests, 0, 1);
    assert_flushing_interval_and_scheduled_batch(&log_event_requests, 1, 2);
    assert_flushing_interval_and_scheduled_batch(&log_event_requests, 2, 4);

    let _ = statsig.shutdown().await;

    std::env::remove_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS");
}

fn assert_flushing_interval_and_scheduled_batch(
    requests: &[wiremock::Request],
    index: usize,
    expected_threshold_interval: u64,
) {
    let json_data = decompress_json(&requests[index].body).expect("Failed to decompress JSON");
    let statsig_metadata = json_data
        .get("statsigMetadata")
        .expect("Missing statsigMetadata field");

    let actual_interval = statsig_metadata
        .get("flushingIntervalMs")
        .and_then(|v| v.as_u64())
        .expect("Missing or invalid flushingIntervalMs field");

    assert!(
        actual_interval >= expected_threshold_interval,
        "Expected loggingInterval >= {}, but got {} at index {}",
        expected_threshold_interval,
        actual_interval,
        index
    );

    assert_eq!(
        statsig_metadata.get("flushType").and_then(|v| v.as_str()),
        Some("scheduled:max_time"),
        "Unexpected flushType to be 'scheduled' at index {}",
        index
    );
}

#[cfg(feature = "with_zstd")]
pub fn decompress_json(compressed: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    let mut decoder = zstd::Decoder::new(compressed)?;
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    let json: Value = serde_json::from_str(&decompressed)?;
    Ok(json)
}

#[cfg(not(feature = "with_zstd"))]
pub fn decompress_json(compressed: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    let json: Value = serde_json::from_str(&decompressed)?;
    Ok(json)
}
