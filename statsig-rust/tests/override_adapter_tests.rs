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
    adapter.override_gate("test_public", false, None);

    let gate = statsig.check_gate(&user, "test_public");
    assert!(!gate);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_feature_gate_overrides() {
    let (statsig, user, adapter) = setup().await;

    assert!(statsig.get_feature_gate(&user, "test_public").value);
    adapter.override_gate("test_public", false, None);

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
        None,
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
        None,
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

    adapter.override_experiment_by_group_name("experiment_with_many_params", "Control", None);

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
        None,
    );

    adapter.override_experiment_by_group_name("experiment_with_many_params", "Control", None);

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
        None,
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
    adapter.override_gate("test_email", true, None);

    assert!(statsig.check_gate(&user, "test_nested_gate_condition"));

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_check_gate_overrides_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    assert!(statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    adapter.override_gate("test_public", false, Some("a_user"));

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

    adapter.override_gate("test_public", false, Some("a_user"));

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

    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
        Some("a_user"),
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

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        Some("a_user"),
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

    adapter.override_experiment_by_group_name(
        "experiment_with_many_params",
        "Control",
        Some("a_user"),
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

    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        Some("a_user"),
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

    adapter.override_gate("test_public", false, None);
    adapter.override_gate("test_public", true, Some("a_user"));

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

    adapter.override_gate("test_public", false, Some("employee_id:12345"));

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

    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-9.87))]),
        Some("employee_id:12345"),
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

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("custom_id_value"))]),
        Some("employee_id:12345"),
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

    adapter.override_experiment_by_group_name(
        "experiment_with_many_params",
        "Test",
        Some("employee_id:12345"),
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

    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("custom_id_value"))]),
        Some("employee_id:12345"),
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

    adapter.override_gate("test_public", false, None); // Name override (lowest precedence)
    adapter.override_gate("test_public", true, Some("12345")); // Custom ID override
    adapter.override_gate("test_public", false, Some("custom_user")); // User ID override (highest precedence)

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

    adapter.override_gate("test_public", false, Some("employee_id:12345"));
    adapter.override_gate("test_public", false, Some("a_user"));

    assert!(!statsig.check_gate(&user_with_custom_id, "test_public"));
    assert!(!statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    statsig.shutdown().await.unwrap();
}
#[tokio::test]
async fn test_remove_gate_override() {
    let (statsig, user, adapter) = setup().await;

    assert!(statsig.check_gate(&user, "test_public"));
    adapter.override_gate("test_public", false, None);

    let gate = statsig.check_gate(&user, "test_public");
    assert!(!gate);

    adapter.remove_gate_override("test_public", None);
    let gate_after_remove = statsig.check_gate(&user, "test_public");
    assert!(gate_after_remove);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_gate_override_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    assert!(statsig.check_gate(&user, "test_public"));
    assert!(statsig.check_gate(&other_user, "test_public"));

    adapter.override_gate("test_public", false, Some("a_user"));

    let gate = statsig.check_gate(&user, "test_public");
    assert!(!gate);

    let other_gate = statsig.check_gate(&other_user, "test_public");
    assert!(other_gate);

    adapter.remove_gate_override("test_public", Some("a_user"));
    let gate_after_remove = statsig.check_gate(&user, "test_public");
    assert!(gate_after_remove);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_dynamic_config_override() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(original.get_f64("foo", 0f64), 1e21);

    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
        None,
    );

    let overridden = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(overridden.get_f64("foo", 0f64), -1.23);
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    adapter.remove_dynamic_config_override("big_number", None);
    let config_after_remove = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(config_after_remove.get_f64("foo", 0f64), 1e21);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_dynamic_config_override_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_dynamic_config(&user, "big_number");
    let other_original = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(original.get_f64("foo", 0f64), 1e21);
    assert_eq!(other_original.get_f64("foo", 0f64), 1e21);

    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
        Some("a_user"),
    );

    let overridden = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(overridden.get_f64("foo", 0f64), -1.23);
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    let other_overridden = statsig.get_dynamic_config(&other_user, "big_number");
    assert_eq!(other_overridden.get_f64("foo", 0f64), 1e21);

    adapter.remove_dynamic_config_override("big_number", Some("a_user"));
    let config_after_remove = statsig.get_dynamic_config(&user, "big_number");
    assert_eq!(config_after_remove.get_f64("foo", 0f64), 1e21);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_experiment_override() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        None,
    );

    let overridden = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    adapter.remove_experiment_override("experiment_with_many_params", None);
    let experiment_after_remove = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        experiment_after_remove.get_string("a_string", "ERR".to_string()),
        "test_2"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_experiment_override_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_experiment(&user, "experiment_with_many_params");
    let other_original = statsig.get_experiment(&other_user, "experiment_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        Some("a_user"),
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

    adapter.remove_experiment_override("experiment_with_many_params", Some("a_user"));
    let experiment_after_remove = statsig.get_experiment(&user, "experiment_with_many_params");
    assert_eq!(
        experiment_after_remove.get_string("a_string", "ERR".to_string()),
        "test_2"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_layer_override() {
    let (statsig, user, adapter) = setup().await;

    let original = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        None,
    );

    let overridden = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(
        overridden.get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(overridden.details.reason, "LocalOverride:Recognized");

    adapter.remove_layer_override("layer_with_many_params", None);
    let layer_after_remove = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(
        layer_after_remove.get_string("a_string", "ERR".to_string()),
        "test_2"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_layer_override_for_id() {
    let (statsig, user, adapter) = setup().await;
    let other_user = StatsigUser::with_user_id("b_user".to_string());

    let original = statsig.get_layer(&user, "layer_with_many_params");
    let other_original = statsig.get_layer(&other_user, "layer_with_many_params");
    assert_eq!(original.get_string("a_string", "ERR".to_string()), "test_2");

    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        Some("a_user"),
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

    adapter.remove_layer_override("layer_with_many_params", Some("a_user"));
    let layer_after_remove = statsig.get_layer(&user, "layer_with_many_params");
    assert_eq!(
        layer_after_remove.get_string("a_string", "ERR".to_string()),
        "test_2"
    );

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_remove_all_overrides() {
    let (statsig, user, adapter) = setup().await;

    adapter.override_gate("test_public", false, None);
    adapter.override_dynamic_config(
        "big_number",
        HashMap::from([("foo".to_string(), json!(-1.23))]),
        None,
    );
    adapter.override_experiment(
        "experiment_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        None,
    );
    adapter.override_layer(
        "layer_with_many_params",
        HashMap::from([("a_string".to_string(), json!("overridden_value"))]),
        None,
    );

    assert!(!statsig.check_gate(&user, "test_public"));
    assert_eq!(
        statsig
            .get_dynamic_config(&user, "big_number")
            .get_f64("foo", 0f64),
        -1.23
    );
    assert_eq!(
        statsig
            .get_experiment(&user, "experiment_with_many_params")
            .get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );
    assert_eq!(
        statsig
            .get_layer(&user, "layer_with_many_params")
            .get_string("a_string", "ERR".to_string()),
        "overridden_value"
    );

    adapter.remove_all_overrides();

    assert!(statsig.check_gate(&user, "test_public"));
    assert_eq!(
        statsig
            .get_dynamic_config(&user, "big_number")
            .get_f64("foo", 0f64),
        1e21
    );
    assert_eq!(
        statsig
            .get_experiment(&user, "experiment_with_many_params")
            .get_string("a_string", "ERR".to_string()),
        "test_2"
    );
    assert_eq!(
        statsig
            .get_layer(&user, "layer_with_many_params")
            .get_string("a_string", "ERR".to_string()),
        "test_2"
    );

    statsig.shutdown().await.unwrap();
}
