mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::json;
use sigstat::output_logger::LogLevel;
use sigstat::{Statsig, StatsigLocalOverrideAdapter, StatsigOptions, StatsigUser};
use std::collections::HashMap;
use std::sync::Arc;

async fn setup() -> (Statsig, StatsigUser, Arc<StatsigLocalOverrideAdapter>) {
    let adapter = Arc::new(StatsigLocalOverrideAdapter::new());
    let user = StatsigUser::with_user_id("a_user".to_string());
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            output_log_level: Some(LogLevel::Debug),
            override_adapter: Some(adapter.clone()),
            environment: Some("development".to_string()),
            ..StatsigOptions::new()
        })),
    );

    statsig.initialize().await.unwrap();

    (statsig, user, adapter)
}

#[tokio::test]
async fn test_check_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(statsig.check_gate(&user, "test_public"));
    adapter.override_gate("test_public", false);

    let gate = statsig.check_gate(&user, "test_public");
    assert!(!gate);
}

#[tokio::test]
async fn test_get_feature_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(statsig.get_feature_gate(&user, "test_public").value);
    adapter.override_gate("test_public", false);

    let gate = statsig.get_feature_gate(&user, "test_public");
    assert!(!gate.value);
    assert_eq!(gate.details.reason, "LocalOverride:Recognized");
}

#[tokio::test]
async fn test_get_dynamic_config_overrides() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(original.get_f64("foo", 0f64), 1e21);

    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
    );

    let overridden = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(overridden.get_f64("foo", 0f64), -1.23);
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");
}

#[tokio::test]
async fn test_get_experiment_overrides() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
    );

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");
}

#[tokio::test]
async fn test_get_layer_overrides() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
    );

    let overridden = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");
}

#[tokio::test]
async fn test_nested_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(!statsig.check_gate(&user, "test_nested_gate_condition"));
    adapter.override_gate("test_email", true);

    assert!(statsig.check_gate(&user, "test_nested_gate_condition"));
}
