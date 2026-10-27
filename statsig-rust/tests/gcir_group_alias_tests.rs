mod utils;

use std::sync::Arc;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::Value;
use statsig_rust::{
    gcir::gcir_formatter::GCIRResponseFormat, ClientInitResponseOptions, HashAlgorithm, Statsig,
    StatsigOptions, StatsigUser,
};

async fn get_gcir(response_format: Option<GCIRResponseFormat>) -> Value {
    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data(
        "tests/data/eval_proj_dcs.json",
    )));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    let response = statsig.get_client_init_response_with_options_as_string(
        &StatsigUser::with_user_id("9"),
        &ClientInitResponseOptions {
            hash_algorithm: Some(HashAlgorithm::None),
            response_format,
            ..Default::default()
        },
    );
    serde_json::from_str(&response).unwrap()
}

#[tokio::test]
async fn test_experiment_group_is_rule_id_for_assigned_group() {
    let gcir = get_gcir(None).await;
    let exp = &gcir["dynamic_configs"]["test_exp_random_id_allocated"];

    assert_eq!(exp["rule_id"], "4qC3kJ5j58KwBAt2VLKL7M");
    assert_eq!(exp["group"], exp["rule_id"]);
    assert_eq!(exp["group_name"], "Test #3");
}

#[tokio::test]
async fn test_experiment_group_is_rule_id_for_targeting_gate() {
    let gcir = get_gcir(None).await;
    let exp = &gcir["dynamic_configs"]["test_exp_5050_targeting"];

    assert_eq!(exp["rule_id"], "targetingGate");
    assert_eq!(exp["group"], "targetingGate");
    assert!(exp.get("group_name").is_none());
}

#[tokio::test]
async fn test_dynamic_config_and_layer_group_is_rule_id() {
    let gcir = get_gcir(None).await;
    let config = &gcir["dynamic_configs"]["test_custom_config"];
    let layer = &gcir["layer_configs"]["test_layer_with_holdout"];

    assert!(config["group"].is_string());
    assert!(layer["group"].is_string());
    assert_eq!(config["group"], config["rule_id"]);
    assert_eq!(layer["group"], layer["rule_id"]);
}

#[tokio::test]
async fn test_group_is_omitted_outside_v1_format() {
    let gcir = get_gcir(Some(
        GCIRResponseFormat::InitializeWithSecondaryExposureMapping,
    ))
    .await;
    let exp = &gcir["dynamic_configs"]["test_exp_random_id_allocated"];

    assert!(exp.get("rule_id").is_some());
    assert!(exp.get("group").is_none());
}
