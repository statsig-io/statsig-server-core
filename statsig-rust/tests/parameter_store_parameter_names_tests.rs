mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::json;
use statsig_rust::{
    OverrideAdapter, Statsig, StatsigLocalOverrideAdapter, StatsigOptions, StatsigUser,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// "a_param_store" in demo_proj_dcs.json is backed by every parameter ref_type:
// static, gate, dynamic_config, experiment and layer.
const MIXED_STORE_NAME: &str = "a_param_store";
const EXPECTED_MIXED_STORE_PARAM_NAMES: [&str; 7] = [
    "a_bool_param",          // experiment
    "a_num_param",           // experiment
    "a_string_param",        // gate
    "an_array_param",        // static value
    "an_object_param",       // layer
    "another_string_param",  // dynamic config
    "brock_an_object_param", // experiment
];

async fn setup_demo_proj(logging_adapter: &Arc<MockEventLoggingAdapter>) -> (Statsig, StatsigUser) {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/demo_proj_dcs.json",
            ))),
            event_logging_adapter: Some(logging_adapter.clone()),
            output_log_level: Some(statsig_rust::output_logger::LogLevel::Error),
            ..StatsigOptions::new()
        })),
    );

    statsig.initialize().await.unwrap();

    (statsig, StatsigUser::with_user_id("a_user_id"))
}

async fn setup_eval_proj_with_overrides() -> (Statsig, StatsigUser, Arc<StatsigLocalOverrideAdapter>)
{
    let adapter = Arc::new(StatsigLocalOverrideAdapter::new());
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            override_adapter: Some(adapter.clone()),
            output_log_level: Some(statsig_rust::output_logger::LogLevel::Error),
            ..StatsigOptions::new()
        })),
    );

    statsig.initialize().await.unwrap();

    (statsig, StatsigUser::with_user_id("a_user"), adapter)
}

#[tokio::test]
async fn test_get_parameter_names_from_store_all_ref_types() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let (statsig, user) = setup_demo_proj(&logging_adapter).await;

    let names = statsig.get_parameter_names_from_store(&user, MIXED_STORE_NAME);

    assert_eq!(names, EXPECTED_MIXED_STORE_PARAM_NAMES.to_vec());

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_parameter_names_is_sorted_and_deterministic() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let (statsig, user) = setup_demo_proj(&logging_adapter).await;

    let first = statsig.get_parameter_names_from_store(&user, MIXED_STORE_NAME);

    let mut sorted = first.clone();
    sorted.sort();
    assert_eq!(first, sorted, "parameter names must be sorted");

    // HashMap iteration order is nondeterministic - repeat to catch unsorted output
    for _ in 0..25 {
        let next = statsig.get_parameter_names_from_store(&user, MIXED_STORE_NAME);
        assert_eq!(first, next, "parameter name ordering must be stable");
    }

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_parameter_names_from_unrecognized_store_is_empty() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let (statsig, user) = setup_demo_proj(&logging_adapter).await;

    let names = statsig.get_parameter_names_from_store(&user, "not_a_real_parameter_store");

    assert!(
        names.is_empty(),
        "unrecognized store should yield an empty list, got {names:?}"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_parameter_names_logs_no_exposures() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let (statsig, user) = setup_demo_proj(&logging_adapter).await;

    let names = statsig.get_parameter_names_from_store(&user, MIXED_STORE_NAME);
    assert!(!names.is_empty());

    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payloads = logging_adapter.logged_payloads.lock().unwrap().clone();
    let event_names: Vec<String> = payloads
        .iter()
        .filter_map(|p| p.events.as_array().cloned())
        .flatten()
        .filter_map(|e| {
            e.get("eventName")
                .and_then(|n| n.as_str())
                .map(str::to_string)
        })
        .filter(|name| name != "statsig::diagnostics")
        .collect();

    let exposures: Vec<&String> = event_names
        .iter()
        .filter(|name| name.contains("_exposure"))
        .collect();

    assert!(
        exposures.is_empty(),
        "expected no exposure events, got {exposures:?}"
    );

    // The only thing the parameter store builder should log is the
    // non-exposed-check counter, never an exposure.
    assert!(
        event_names
            .iter()
            .all(|name| name == "statsig::non_exposed_checks"),
        "unexpected events logged: {event_names:?}"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_parameter_names_respects_parameter_store_overrides() {
    let (statsig, user, adapter) = setup_eval_proj_with_overrides().await;

    assert_eq!(
        statsig.get_parameter_names_from_store(&user, "test_parameter_store"),
        vec!["bool_param".to_string()]
    );

    adapter.override_parameter_store(
        "test_parameter_store",
        HashMap::from([
            ("zeta_param".to_string(), json!(true)),
            ("alpha_param".to_string(), json!("a_string")),
        ]),
        Some("a_user"),
    );

    // Must go through the user-aware path so the override replaces the whole map
    assert_eq!(
        statsig.get_parameter_names_from_store(&user, "test_parameter_store"),
        vec!["alpha_param".to_string(), "zeta_param".to_string()]
    );

    // A different user is unaffected by the user-scoped override
    let other_user = StatsigUser::with_user_id("another_user");
    assert_eq!(
        statsig.get_parameter_names_from_store(&other_user, "test_parameter_store"),
        vec!["bool_param".to_string()]
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_parameter_store_get_parameter_names() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let (statsig, user) = setup_demo_proj(&logging_adapter).await;

    let store = statsig.get_parameter_store_for_user(&user, MIXED_STORE_NAME);
    assert_eq!(
        store.get_parameter_names(),
        EXPECTED_MIXED_STORE_PARAM_NAMES.to_vec()
    );

    let missing = statsig.get_parameter_store_for_user(&user, "nope");
    assert!(missing.get_parameter_names().is_empty());

    statsig.shutdown().await.unwrap();
}
