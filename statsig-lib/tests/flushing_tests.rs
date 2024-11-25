mod utils;

use sigstat::{output_logger::LogLevel, Statsig, StatsigOptions, StatsigUser};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use utils::{
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
    mock_specs_adapter::MockSpecsAdapter,
};

const SDK_KEY: &str = "secret-key";

async fn setup(delay_ms: u64) -> (MockScrapi, Statsig) {
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
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        })),
    );

    (mock_scrapi, statsig)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (_, statsig) = setup(900 /* delay_ms */).await;

    statsig.initialize().await.unwrap();

    let start = Instant::now();
    for i in 0..10000 {
        let user = StatsigUser::with_user_id(format!("test_user_{}", i));
        let gate = statsig.check_gate(&user, "test_public");
        assert_eq!(gate, true);
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
    let (_, statsig) = setup(2000 /* delay_ms */).await;

    statsig.initialize().await.unwrap();

    let start = Instant::now();
    for i in 0..5000 {
        let user = StatsigUser::with_user_id(format!("test_user_{}", i));
        let gate = statsig.check_gate(&user, "test_public");
        assert_eq!(gate, true);
    }

    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000);
    println!("check_gate: {:.2} ms", duration.as_millis());

    let _ = statsig
        .shutdown_with_timeout(Duration::from_millis(100))
        .await;

    let overall_duration = start.elapsed();
    println!("overall: {:.2} ms", overall_duration.as_millis());
    assert!(overall_duration.as_millis() < 2000);
}

#[tokio::test]
async fn test_all_events_get_flushed() {
    let (mock_scrapi, statsig) = setup(0 /* delay_ms */).await;

    statsig.initialize().await.unwrap();

    for i in 0..5000 {
        let user = StatsigUser::with_user_id(format!("test_user_{}", i));
        let gate = statsig.check_gate(&user, "test_public");
        assert_eq!(gate, true);
    }

    // sleep(Duration::from_millis(100)).await;
    let start = Instant::now();
    statsig
        .shutdown_with_timeout(Duration::from_millis(3000))
        .await
        .unwrap();

    assert_eventually_eq!(
        || mock_scrapi.get_logged_event_count(),
        5001,
        Duration::from_secs(1)
    )
    .await;

    let duration = start.elapsed();
    println!("shutdown: {:.2} ms", duration.as_millis());
}
