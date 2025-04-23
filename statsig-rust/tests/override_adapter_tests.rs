mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::json;
use statsig_rust::output_logger::LogLevel;
use statsig_rust::{
    OverrideAdapter, Statsig, StatsigLocalOverrideAdapter, StatsigOptions, StatsigUser,
};
use std::collections::HashMap;
use std::sync::Arc;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

async fn setup() -> (Statsig, StatsigUser, Arc<StatsigLocalOverrideAdapter>) {
    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let adapter = Arc::new(StatsigLocalOverrideAdapter::new());
    let user = StatsigUser::with_user_id("a_user".to_string());
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            output_log_level: Some(LogLevel::Debug),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            override_adapter: Some(adapter.clone()),
            environment: Some("development".to_string()),
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
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

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_feature_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(statsig.get_feature_gate(&user, "test_public").value);
    adapter.override_gate("test_public", false);

    let gate = statsig.get_feature_gate(&user, "test_public");
    assert!(!gate.value);
    assert_eq!(gate.details.reason, "LocalOverride:Recognized");

    statsig.shutdown().await.unwrap();
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

    statsig.shutdown().await.unwrap();
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

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_by_group_name() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment_by_group_name("experiment_with_many_params", "Control");

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "control"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_in_order() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
    );

    adapter.override_experiment_by_group_name("experiment_with_many_params", "Control");

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "control"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    statsig.shutdown().await.unwrap();
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

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_nested_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(!statsig.check_gate(&user, "test_nested_gate_condition"));
    adapter.override_gate("test_email", true);

    assert!(statsig.check_gate(&user, "test_nested_gate_condition"));

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_check_gate_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    assert!(statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    adapter.override_gate_for_id("test_public", "a_user", false);

    let gate = statsig.check_gate(&user, "test_public");
    assert!(!gate);

    let other_gate = statsig.check_gate(&other_user, "test_public");
    assert!(other_gate);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_feature_gate_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    assert!(statsig.get_feature_gate(&user, "test_public").value);
    assert!(statsig.get_feature_gate(&other_user, "test_public").value);

    adapter.override_gate_for_id("test_public", "a_user", false);

    let gate = statsig.get_feature_gate(&user, "test_public");
    assert!(!gate.value);
    assert_eq!(gate.details.reason, "LocalOverride:Recognized");

    let other_gate = statsig.get_feature_gate(&other_user, "test_public");
    assert!(other_gate.value);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_dynamic_config_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_dynamic_config(&user, "big_number");
    let other_original = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(original.get_f64("foo", 0f64), 1e21);
    assert_eq!(other_original.get_f64("foo", 0f64), 1e21);

    adapter.override_dynamic_config_for_id(
        "big_number",
        "a_user",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
    );

    let overridden = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(overridden.get_f64("foo", 0f64), -1.23);
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(other_overridden.get_f64("foo", 0f64), 1e21);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    let other_original = statsig.get_experiment(&other_user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment_for_id(
        "experiment_with_many_params",
        "a_user",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
    );

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_experiment(&other_user, "experiment_with_many_params");
    assert_eq!(
        other_overridden.get_string("a_string", "ERR".to_string()),
        other_original.get_string("a_string", "ERR".to_string())
    );
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_by_group_name_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    let other_original = statsig.get_experiment(&other_user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment_by_group_name_for_id(
        "experiment_with_many_params",
        "a_user",
        "Control",
    );

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "control"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_experiment(&other_user, "experiment_with_many_params");
    assert_eq!(
        other_overridden.get_string("a_string", "ERR".to_string()),
        other_original.get_string("a_string", "ERR".to_string())
    );
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_layer_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_layer(&user, "layer_with_many_params");
    let other_original = statsig.get_layer(&other_user, "layer_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_layer_for_id(
        "layer_with_many_params",
        "a_user",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
    );

    let overridden = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_layer(&other_user, "layer_with_many_params");
    assert_eq!(
        other_overridden.get_string("a_string", "ERR".to_string()),
        other_original.get_string("a_string", "ERR".to_string())
    );
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_id_override_precedence_over_name() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    adapter.override_gate("test_public", false);
    adapter.override_gate_for_id("test_public", "a_user", true);

    let gate = statsig.check_gate(&user, "test_public");
    assert!(gate);

    let other_gate = statsig.check_gate(&other_user, "test_public");
    assert!(!other_gate);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_check_gate_overrides_for_custom_id() {
    let (statsig, user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    assert!(statsig.check_gate(&user_with_custom_id, "test_public"));
    assert!(statsig.check_gate(&user, "test_public"));

    adapter.override_gate_for_id("test_public", "employee_id:12345", false);

    let gate = statsig.check_gate(&user_with_custom_id, "test_public");
    assert!(!gate);

    let other_gate = statsig.check_gate(&user, "test_public");
    assert!(other_gate);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_dynamic_config_overrides_for_custom_id() {
    let (statsig, _user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    let other_user = StatsigUser::with_user_id("other_user".to_string());

    let original = statsig.get_dynamic_config(&user_with_custom_id, "big_number");
    let other_original = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(original.get_f64("foo", 0f64), 1e21);
    assert_eq!(other_original.get_f64("foo", 0f64), 1e21);

    adapter.override_dynamic_config_for_id(
        "big_number",
        "employee_id:12345",
        HashMap::from([("foo".to_string(), json!(-9.87))]),
    );

    let overridden = statsig.get_dynamic_config(&user_with_custom_id, "big_number");
    assert_eq!(overridden.get_f64("foo", 0f64), -9.87);
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(other_overridden.get_f64("foo", 0f64), 1e21);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_for_custom_id() {
    let (statsig, _user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    let original = statsig.get_experiment(&user_with_custom_id, "experiment_with_many_params");
    assert_eq!(
        original.get_string("a_string", "ERR".to_string()),
        "control"
    );

    adapter.override_experiment_for_id(
        "experiment_with_many_params",
        "employee_id:12345",
        HashMap::from([("a_string".to_string(), json!("custom_id_value"))]),
    );

    let overridden = statsig.get_experiment(&user_with_custom_id, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "custom_id_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_experiment_overrides_by_group_name_for_custom_id() {
    let (statsig, _user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    let original = statsig.get_experiment(&user_with_custom_id, "experiment_with_many_params");
    assert_eq!(
        original.get_string("a_string", "ERR".to_string()),
        "control"
    );

    adapter.override_experiment_by_group_name_for_id(
        "experiment_with_many_params",
        "employee_id:12345",
        "Test",
    );

    let overridden = statsig.get_experiment(&user_with_custom_id, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "test_1"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_layer_overrides_for_custom_id() {
    let (statsig, _user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    let other_user = StatsigUser::with_user_id("other_user".to_string());

    let original = statsig.get_layer(&user_with_custom_id, "layer_with_many_params");
    let other_original = statsig.get_layer(&other_user, "layer_with_many_params");
    assert_eq!(
        original.get_string("a_string", "ERR".to_string()),
        "control"
    );

    adapter.override_layer_for_id(
        "layer_with_many_params",
        "employee_id:12345",
        HashMap::from([("a_string".to_string(), json!("custom_id_value"))]),
    );

    let overridden = statsig.get_layer(&user_with_custom_id, "layer_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "custom_id_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_layer(&other_user, "layer_with_many_params");
    assert_eq!(
        other_overridden.get_string("a_string", "ERR".to_string()),
        other_original.get_string("a_string", "ERR".to_string())
    );
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_override_precedence() {
    let (statsig, _user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    adapter.override_gate("test_public", false); // Name override (lowest precedence)
    adapter.override_gate_for_id("test_public", "12345", true); // Custom ID override
    adapter.override_gate_for_id("test_public", "custom_user", false); // User ID override (highest precedence)

    let gate = statsig.check_gate(&user_with_custom_id, "test_public");
    assert!(!gate);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_gate_override_does_not_spill_over() {
    let (statsig, user, adapter) = setup().await;

    let user_with_custom_id = StatsigUser::with_custom_ids(HashMap::from([(
        "employee_id".to_string(),
        "employee_id:12345".to_string(),
    )]));

    let other_user = StatsigUser::with_user_id("other_user".to_string());

    assert!(statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&user_with_custom_id, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    adapter.override_gate_for_id("test_public", "employee_id:12345", false);
    adapter.override_gate_for_id("test_public", "a_user", false);

    assert!(!statsig.check_gate(&user_with_custom_id, "test_public"));
    assert!(!statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    statsig.shutdown().await.unwrap();
}
