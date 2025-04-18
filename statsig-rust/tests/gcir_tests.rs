mod utils;

use std::sync::Arc;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use assert_json_diff::assert_json_eq;
use lazy_static::lazy_static;
use serde_json::{json, Value};
use statsig_rust::{
    ClientInitResponseOptions, HashAlgorithm, Statsig, StatsigOptions, StatsigUser,
};

lazy_static! {
    static ref USER: StatsigUser =  StatsigUser {
        user_id: Some("9".into()),
        app_version: Some("1.3".into()),
        user_agent: Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into(),
        ),
        ip: Some("1.0.0.0".into()),
        locale: Some("en_US".into()),
        ..StatsigUser::with_user_id("a_user_id".into())
    };
}

async fn setup() -> Value {
    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data(
        "tests/data/eval_proj_dcs.json",
    )));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    let response = statsig.get_client_init_response_with_options(
        &USER,
        &ClientInitResponseOptions {
            hash_algorithm: Some(HashAlgorithm::None),
            client_sdk_key: None,
            include_local_overrides: Some(false),
        },
    );
    let json = serde_json::to_string(&response).unwrap();
    serde_json::from_str(&json).unwrap()
}

#[tokio::test]
async fn test_public_gate() {
    let json_obj = setup().await;

    let gate: &Value = json_obj
        .get("feature_gates")
        .unwrap()
        .get("test_public")
        .unwrap();

    assert_json_eq!(
        gate,
        json!({
            "name": "test_public",
            "value": true,
            "rule_id": "6X3qJgyfwA81IJ2dxI7lYp",
            "id_type": "userID",
            "secondary_exposures": []
        })
    );
}

#[tokio::test]
async fn test_nested_gate_condition() {
    let json_obj = setup().await;

    let gate: &Value = json_obj
        .get("feature_gates")
        .unwrap()
        .get("test_nested_gate_condition")
        .unwrap();

    assert_json_eq!(
        gate,
        json!({
            "name": "test_nested_gate_condition",
            "value": true,
            "rule_id": "6MlXHRavmo1ujM1NkZNjhQ",
            "id_type": "userID",
            "secondary_exposures": [
                {
                    "gate": "test_email", // todo: hash these
                    "gateValue": "false",
                    "ruleID": "default"
                },
                {
                    "gate": "test_environment_tier", // todo: hash these
                    "gateValue": "false",
                    "ruleID": "default"
                }
            ]
        })
    );
}

#[tokio::test]
async fn test_targeted_exp_in_layer_with_holdout() {
    let json_obj = setup().await;

    let experiment: &Value = json_obj
        .get("dynamic_configs")
        .unwrap()
        .get("targeted_exp_in_layer_with_holdout")
        .unwrap();

    assert_json_eq!(
        experiment,
        json!({
            "name": "targeted_exp_in_layer_with_holdout",
            "value": {
                "exp_val": "shipped_test",
                "layer_val": "layer_default"
            },
            "rule_id": "layerAssignment",
            "group": "layerAssignment",
            "is_device_based": false,
            "id_type": "userID",
            "is_experiment_active": true,
            "is_user_in_experiment": false,
            "is_in_layer": true,
            "explicit_parameters": [
                "exp_val"
            ],
            "secondary_exposures": [
                {
                    "gate": "global_holdout",
                    "gateValue": "false",
                    "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"
                },
                {
                    "gate": "layer_holdout",
                    "gateValue": "false",
                    "ruleID": "2bAVp6R3C85vCYrR6be36n:10.00:5"
                }
            ]
        })
    );
}

#[tokio::test]
async fn test_targeted_exp_in_unlayered_with_holdout() {
    let json_obj = setup().await;

    let config: &Value = json_obj
        .get("dynamic_configs")
        .unwrap()
        .get("targeted_exp_in_unlayered_with_holdout")
        .unwrap();

    assert_json_eq!(
        config,
        json!({
          "group": "targetingGate",
          "id_type": "userID",
          "is_device_based": false,
          "is_experiment_active": true,
          "is_user_in_experiment": false,
          "name": "targeted_exp_in_unlayered_with_holdout",
          "rule_id": "targetingGate",
          "secondary_exposures": [
            {
              "gate": "global_holdout",
              "gateValue": "false",
              "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"
            },
            {
              "gate": "exp_holdout",
              "gateValue": "false",
              "ruleID": "1rEqLOpCROaRafv7ubGgax"
            },
            {
              "gate": "test_50_50",
              "gateValue": "false",
              "ruleID": "6U5gYSQ2jRCDWvfPzKSQY9"
            }
          ],
          "value": {}
        })
    );
}

#[tokio::test]
async fn test_exp_5050_targeting() {
    let json_obj = setup().await;

    let experiment: &Value = json_obj
        .get("dynamic_configs")
        .unwrap()
        .get("test_exp_5050_targeting")
        .unwrap();

    assert_json_eq!(
        experiment,
        json!({
            "name": "test_exp_5050_targeting",
            "value": {},
            "rule_id": "targetingGate",
            "group": "targetingGate",
            "is_device_based": false,
            "id_type": "userID",
            "is_experiment_active": true,
            "is_user_in_experiment": false,
            "secondary_exposures": [
                {
                    "gate": "global_holdout",
                    "gateValue": "false",
                    "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"
                },
                {
                    "gate": "test_50_50",
                    "gateValue": "false",
                    "ruleID": "6U5gYSQ2jRCDWvfPzKSQY9"
                }
            ]
        })
    );
}

#[tokio::test]
async fn test_targetting_with_capital_letter_gate() {
    let json_obj = setup().await;

    let experiment: &Value = json_obj
        .get("dynamic_configs")
        .unwrap()
        .get("test_targetting_with_capital_letter_gate")
        .unwrap();

    assert_json_eq!(
        experiment,
        json!({
            "name": "test_targetting_with_capital_letter_gate",
            "value": {
                "Result": "This is right"
            },
            "rule_id": "74pyYBYPZ5Xly55E6J3lEq",
            "group": "74pyYBYPZ5Xly55E6J3lEq",
            "group_name": "Test",
            "is_device_based": false,
            "id_type": "userID",
            "is_experiment_active": true,
            "is_user_in_experiment": true,
            "secondary_exposures": [
                {
                    "gate": "global_holdout",
                    "gateValue": "false",
                    "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"
                },
                {
                    "gate": "test_putting_CAPITAL_letters_in_id",
                    "gateValue": "true",
                    "ruleID": "3Gv6T9YIObRmqZV5nAv0fO"
                }
            ]
        })
    );
}

#[tokio::test]
async fn test_layer_with_many_params() {
    let json_obj = setup().await;

    let layer: &Value = json_obj
        .get("layer_configs")
        .unwrap()
        .get("layer_with_many_params")
        .unwrap();

    assert_json_eq!(
        layer,
        json!({
            "name": "layer_with_many_params",
            "value": {
                "a_string": "layer",
                "another_string": "layer_default",
                "a_number": 799,
                "a_bool": false,
                "an_object": {
                    "value": "layer_default"
                },
                "an_array": [
                    "layer_default"
                ],
                "another_bool": true,
                "another_number": 0
            },
            "id_type": "userID",
            "rule_id": "default",
            "group": "default",
            "is_device_based": false,
            "explicit_parameters": [],
            "secondary_exposures": [],
            "undelegated_secondary_exposures": []
        })
    );
}

#[tokio::test]
async fn test_layer_with_no_exp() {
    let json_obj = setup().await;

    let layer: &Value = json_obj
        .get("layer_configs")
        .unwrap()
        .get("test_layer_with_no_exp")
        .unwrap();

    assert_json_eq!(
        layer,
        json!({
            "name": "test_layer_with_no_exp",
            "value": {
                "a_param": "foo"
            },
            "id_type": "userID",
            "rule_id": "default",
            "group": "default",
            "is_device_based": false,
            "explicit_parameters": [],
            "secondary_exposures": [],
            "undelegated_secondary_exposures": []
        })
    );
}

#[tokio::test]
async fn test_autotune() {
    let json_obj = setup().await;

    let experiment: &Value = json_obj
        .get("dynamic_configs")
        .unwrap()
        .get("test_autotune")
        .unwrap();

    assert_json_eq!(
        experiment,
        json!({
            "name": "test_autotune",
            "value": {},
            "rule_id": "5380HnrABE4p869fZhtUV9",
            "group": "5380HnrABE4p869fZhtUV9",
            "group_name": "black",
            "is_device_based": false,
            "id_type": "userID",
            "secondary_exposures": []
        })
    );
}
