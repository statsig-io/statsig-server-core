mod utils;

use crate::utils::mock_observability_client::MockObservabilityClient;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use more_asserts::assert_gt;
use serial_test::serial;
use statsig_rust::networking::NetworkError;
use statsig_rust::output_logger::LogLevel;
use statsig_rust::{ObservabilityClient, Statsig, StatsigErr, StatsigOptions, StatsigUser};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;

async fn setup(
    options: StatsigOptions,
) -> (
    Statsig,
    Arc<MockEventLoggingAdapter>,
    Arc<MockObservabilityClient>,
) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));

    let obs_client = Arc::new(MockObservabilityClient::new());
    let obs_client_dyn: Arc<dyn ObservabilityClient> = obs_client.clone();

    let mut options = options;
    options.specs_adapter = Some(specs_adapter);
    options.event_logging_adapter = Some(logging_adapter.clone());
    options.disable_user_agent_parsing = Some(true);
    options.disable_country_lookup = Some(true);
    options.output_log_level = Some(LogLevel::Debug);
    options.observability_client = Some(Arc::downgrade(&obs_client_dyn));

    let uuid = uuid::Uuid::new_v4();
    let statsig = Statsig::new(&format!("secret-{uuid}"), Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (statsig, logging_adapter, obs_client)
}

async fn teardown(statsig: Option<Statsig>) {
    std::env::remove_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS");
    std::env::remove_var("STATSIG_TEST_OVERRIDE_MAX_LOG_EVENT_RETRIES");

    if let Some(statsig) = statsig {
        let _ = statsig.shutdown().await;
    }
}

#[tokio::test]
#[serial]
async fn test_limit_flushing() {
    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);
    options.event_logging_max_pending_batch_queue_size = Some(60);

    let (statsig, logging_adapter, _) = setup(options).await;

    log_some_events(&statsig, 456);

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

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_scheduled_flush_batch_size() {
    const MAX_EVENTS: usize = 5;

    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(MAX_EVENTS as u32);
    options.event_logging_max_pending_batch_queue_size = Some(2);

    let (statsig, logging_adapter, _) = setup(options).await;

    // trigger failure backoff
    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::CustomError("test error".into()));

    log_some_events(&statsig, MAX_EVENTS);

    // begin accepting events
    logging_adapter.on_log_notify.notified().await;
    *logging_adapter.mocked_log_events_result.lock().unwrap() = Ok(true);

    assert_eventually_eq!(
        || logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        MAX_EVENTS as u64
    );

    let req = logging_adapter.logged_payloads.lock().unwrap().remove(0);
    assert_eq!(
        req.statsig_metadata
            .get("flushType")
            .and_then(|v| v.as_str()),
        Some("scheduled:full_batch")
    );

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_scheduled_flush_max_time() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS", "1");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);
    options.event_logging_max_pending_batch_queue_size = Some(2);

    let (statsig, logging_adapter, _) = setup(options).await;

    // trigger failure backoff
    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::CustomError("test error".into()));

    let user = StatsigUser::with_user_id("user_1");
    for _ in 0..5 {
        statsig.log_event(&user, "test_event", None, None);
    }

    // begin accepting events
    wait_for_log_notify(&logging_adapter).await;
    *logging_adapter.mocked_log_events_result.lock().unwrap() = Ok(true);

    assert_eventually_eq!(
        || logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        5
    );

    let req = logging_adapter.logged_payloads.lock().unwrap().remove(0);
    assert_eq!(
        req.statsig_metadata
            .get("flushType")
            .and_then(|v| v.as_str()),
        Some("scheduled:max_time")
    );

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_scheduled_flush_failures() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_LOG_EVENT_RETRIES", "2");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);

    let (statsig, logging_adapter, obs_client) = setup(options).await;

    // get into failure backoff
    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::CustomError("test error".into()));

    let user = StatsigUser::with_user_id("user_1");
    statsig.log_event(&user, "test_event", None, None);

    wait_for_log_notify(&logging_adapter).await; // first attempt
    wait_for_log_notify(&logging_adapter).await; // second attempt
    assert_eq!(logging_adapter.logged_event_count.load(Ordering::SeqCst), 0);

    assert_eventually!(|| {
        let count = obs_client.error_calls.lock().ok().map(|c| c.len());
        count.is_some() && count.unwrap() >= 1
    });

    let error = obs_client.error_calls.lock().unwrap().remove(0);
    assert_eq!(error.0, "statsig::log_event_failed");

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_requeue_dropped_events() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS", "1");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(5);
    options.event_logging_max_pending_batch_queue_size = Some(1);

    let (statsig, logging_adapter, obs_client) = setup(options).await;

    // get into failure backoff
    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::CustomError("test error".into()));

    let user = StatsigUser::with_user_id("user_1");
    statsig.log_event(&user, "test_event", None, None);

    wait_for_log_notify(&logging_adapter).await;
    assert_eq!(logging_adapter.logged_event_count.load(Ordering::SeqCst), 0);
    assert_eq!(obs_client.error_calls.lock().unwrap().len(), 0);

    for _ in 0..50 {
        statsig.log_event(&user, "test_event", None, None);
    }

    assert_eventually!(|| {
        let count = obs_client.error_calls.lock().ok().map(|c| c.len());
        count.is_some() && count.unwrap() > 1
    });

    let error = obs_client.error_calls.lock().unwrap().remove(0);
    assert_eq!(error.0, "statsig::log_event_dropped_event_count");

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_high_qps_dropped_events() {
    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(10);
    options.event_logging_max_pending_batch_queue_size = Some(2);

    let (statsig, logging_adapter, obs_client) = setup(options).await;

    for i in 0..1000 {
        let user = StatsigUser::with_user_id(format!("user_{i}"));
        let _ = statsig.check_gate(&user, &format!("a_gate_{i}"));
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

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_non_retryable_failure_drops_events() {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_FLUSH_INTERVAL_MS", "1");
    std::env::set_var("STATSIG_TEST_OVERRIDE_MAX_LOG_EVENT_RETRIES", "9999"); // retry forever

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(5);
    options.event_logging_max_pending_batch_queue_size = Some(2);

    let (statsig, logging_adapter, obs_client) = setup(options).await;

    // get into failure backoff
    *logging_adapter.mocked_log_events_result.lock().unwrap() =
        Err(StatsigErr::NetworkError(NetworkError::RequestNotRetryable(
            "test_url".to_string(),
            Some(0),
            "test error".to_string(),
        )));

    let user = StatsigUser::with_user_id("user_1");
    statsig.log_event(&user, "test_event", None, None);

    wait_for_log_notify(&logging_adapter).await;
    assert_eq!(logging_adapter.logged_event_count.load(Ordering::SeqCst), 0);

    assert_eventually!(|| {
        let count = obs_client.error_calls.lock().ok().map(|c| c.len());
        count.is_some() && count.unwrap() >= 1
    });
    assert_eq!(logging_adapter.times_called.load(Ordering::SeqCst), 1);

    let error = match obs_client.error_calls.lock() {
        Ok(calls) => calls
            .iter()
            .find(|(e, _)| e == "statsig::log_event_failed")
            .cloned(),
        Err(_) => None,
    };
    assert!(error.is_some());

    teardown(Some(statsig)).await;
}

#[tokio::test]
#[serial]
async fn test_logging_behavior_when_network_is_disabled() {
    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(5);
    options.disable_network = Some(true);
    options.event_logging_max_pending_batch_queue_size = Some(2);
    let (statsig, logging_adapter, obs_client) = setup(options).await;
    let user = StatsigUser::with_user_id("user_1");
    statsig.log_event(&user, "test_event", None, None);
    statsig.flush_events().await;
    assert!(
        logging_adapter
            .times_called
            .fetch_or(u64::MAX, Ordering::SeqCst)
            == 1
    );
    // Verify observability client is not called
    assert!(obs_client.error_calls.lock().unwrap().is_empty())
}

async fn wait_for_log_notify(logging_adapter: &MockEventLoggingAdapter) {
    tokio::select! {
        _ = logging_adapter.on_log_notify.notified() => {
            // done waiting
        }
        _ = tokio::time::sleep(Duration::from_millis(1000)) => {
            panic!("Timeout waiting for log notify");
        }
    }
}

fn log_some_events(statsig: &Statsig, count: usize) {
    let user = StatsigUser::with_user_id("user_1");
    for _ in 0..count {
        statsig.log_event(&user, "test_event", None, None);
    }
}
