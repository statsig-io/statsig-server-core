mod utils;

use crate::utils::mock_observability_client::MockObservabilityClient;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use more_asserts::assert_gt;
use statsig_rust::output_logger::LogLevel;
use statsig_rust::{ObservabilityClient, Statsig, StatsigErr, StatsigOptions, StatsigUser};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;

async fn setup(options: StatsigOptions) -> (Statsig, Arc<MockEventLoggingAdapter>) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));

    let mut options = options;
    options.specs_adapter = Some(specs_adapter);
    options.event_logging_adapter = Some(logging_adapter.clone());
    options.disable_user_agent_parsing = Some(true);
    options.disable_country_lookup = Some(true);
    options.output_log_level = Some(LogLevel::Debug);

    let uuid = uuid::Uuid::new_v4();
    let statsig = Statsig::new(&format!("secret-{}", uuid), Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (statsig, logging_adapter)
}

fn teardown() {
    std::env::remove_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS");
}

#[tokio::test]
async fn test_limit_flushing() {
    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);
    options.event_logging_max_pending_batch_queue_size = Some(60);

    let (statsig, logging_adapter) = setup(options).await;

    for i in 0..456 {
        let user = StatsigUser::with_user_id(format!("user_{}", i));
        let _ = statsig.check_gate(&user, "test_public");
    }

    assert_eventually!(|| {
        let count = logging_adapter.logged_event_count.load(Ordering::SeqCst);
        count > 0 && count < 456 // logged some but not all
    });

    statsig.shutdown().await.unwrap();

    // logged all events
    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        456
    );

    teardown();
}

#[tokio::test]
async fn test_scheduled_flushing() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);

    let (statsig, logging_adapter) = setup(options).await;

    let user = StatsigUser::with_user_id("user_1".into());
    let _ = statsig.check_gate(&user, "test_public");

    assert_eventually_eq!(
        || logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    statsig.shutdown().await.unwrap();

    teardown();
}

#[tokio::test]
async fn test_scheduled_flush_failures() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);

    let obs_client = Arc::new(MockObservabilityClient::new());
    let obs_client_dyn: Arc<dyn ObservabilityClient> = obs_client.clone();
    options.observability_client = Some(Arc::downgrade(&obs_client_dyn));

    let (statsig, logging_adapter) = setup(options).await;

    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::CustomError("test error".into()));

    let user = StatsigUser::with_user_id("user_1".into());
    let _ = statsig.check_gate(&user, "test_public");

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(logging_adapter.logged_event_count.load(Ordering::SeqCst), 0);

    let error = obs_client.error_calls.lock().unwrap().remove(0);
    assert_eq!(error.0, "statsig::log_event_failed");

    statsig.shutdown().await.unwrap();

    teardown();
}

#[tokio::test]
async fn test_high_qps_dropped_events() {
    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);
    options.event_logging_max_pending_batch_queue_size = Some(2);

    let obs_client = Arc::new(MockObservabilityClient::new());
    let obs_client_dyn: Arc<dyn ObservabilityClient> = obs_client.clone();
    options.observability_client = Some(Arc::downgrade(&obs_client_dyn));

    let (statsig, logging_adapter) = setup(options).await;

    for i in 0..1000 {
        let user = StatsigUser::with_user_id(format!("user_{}", i));
        let _ = statsig.check_gate(&user, &format!("a_gate_{}", i));
    }

    statsig.flush_events().await;

    assert_gt!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        0
    );

    assert_eventually_eq!(
        || {
            let mut calls = match obs_client.error_calls.lock() {
                Ok(calls) => calls,
                Err(_) => return None,
            };

            if calls.is_empty() {
                return None;
            }

            let error = calls.remove(0);
            Some(error.0)
        },
        Some("statsig::log_event_dropped_event_count".to_string())
    );

    statsig.shutdown().await.unwrap();

    teardown();
}
