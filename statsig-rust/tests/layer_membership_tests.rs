mod utils;

use std::collections::HashMap;

use serde_json::{json, Value};
use statsig_rust::{
    evaluation::evaluator_context::IdListResolution, specs_response::spec_types::SpecsResponseFull,
    StatsigUser,
};

use utils::gcir::generate_v1_response;

fn layer_response(
    rule_id: &str,
    delegate_marker: Option<bool>,
    outer_marker: Option<bool>,
    new_layer_eval: bool,
    allocate: bool,
) -> Value {
    let specs: SpecsResponseFull = serde_json::from_value(json!({
        "feature_gates": {},
        "dynamic_configs": {
            "experiment": {
                "type": "dynamic_config", "entity": "experiment", "salt": "experiment",
                "defaultValue": {}, "enabled": true, "idType": "userID", "isActive": true,
                "explicitParameters": ["checkout"],
                "rules": [{
                    "name": "Test", "id": rule_id, "groupName": "Test",
                    "passPercentage": 100, "idType": "userID", "conditions": [],
                    "returnValue": {"checkout": "new"}, "isExperimentGroup": delegate_marker
                }]
            }
        },
        "layer_configs": {
            "layer": {
                "type": "dynamic_config", "entity": "layer", "salt": "layer",
                "defaultValue": {"checkout": "default"}, "enabled": true,
                "idType": "userID", "isActive": true, "useNewLayerEval": new_layer_eval,
                "rules": [{
                    "name": "Assignment", "id": "outer-assignment", "idType": "userID",
                    "conditions": if allocate {json!([])} else {json!(["never"])}, "passPercentage": 100,
                    "returnValue": {}, "configDelegate": "experiment",
                    "isExperimentGroup": outer_marker
                }]
            }
        },
        "condition_map": {"never": {"type": "unit_id", "idType": "userID", "operator": "any", "targetValue": ["ineligible"]}},
        "experiment_to_layer": {"experiment": "layer"},
        "has_updates": true, "time": 1
    }))
    .unwrap();
    let response = generate_v1_response(
        &StatsigUser::with_user_id("test-user"),
        &specs,
        IdListResolution::MapLookup(&HashMap::new()),
        None,
    )
    .unwrap();
    serde_json::to_value(response).unwrap()
}

#[test]
fn delegated_membership_comes_from_the_experiment_not_the_outer_rule() {
    for new_layer_eval in [false, true] {
        for (rule_id, delegate_marker) in [
            ("group-test", Some(true)),
            ("0", Some(true)),
            ("group-test:override", None),
            ("launchedGroup", None),
            ("group-test", Some(false)),
        ] {
            for outer_marker in [None, Some(false), Some(true)] {
                let response =
                    layer_response(rule_id, delegate_marker, outer_marker, new_layer_eval, true);
                let experiment = &response["dynamic_configs"]["experiment"];
                let layer = &response["layer_configs"]["layer"];
                assert_eq!(experiment["value"], json!({"checkout": "new"}));
                assert_eq!(layer["value"], experiment["value"]);
                assert_eq!(layer["allocated_experiment_name"], "experiment");
                assert_eq!(
                    experiment["is_user_in_experiment"],
                    delegate_marker.unwrap_or(false)
                );
                assert_eq!(
                    layer["is_user_in_experiment"], experiment["is_user_in_experiment"],
                    "new_layer_eval={new_layer_eval}, rule={rule_id}, delegate={delegate_marker:?}, outer={outer_marker:?}"
                );
                assert_eq!(layer["is_experiment_active"], true);
                assert_eq!(layer["explicit_parameters"], json!(["checkout"]));
                assert_eq!(layer["group_name"], "Test");
                assert_eq!(
                    layer["rule_id"],
                    if new_layer_eval {
                        "outer-assignment"
                    } else {
                        rule_id
                    }
                );
                if new_layer_eval {
                    assert_eq!(
                        layer["parameter_rule_ids"],
                        json!({"checkout": "outer-assignment"})
                    );
                }
            }
        }
    }
}

#[test]
fn unallocated_layers_omit_membership_even_when_the_outer_rule_is_marked() {
    for new_layer_eval in [false, true] {
        let response = layer_response("group-test", Some(true), Some(true), new_layer_eval, false);
        let layer = &response["layer_configs"]["layer"];
        assert_eq!(layer["value"], json!({"checkout": "default"}));
        assert!(layer.get("allocated_experiment_name").is_none());
        assert!(layer.get("is_user_in_experiment").is_none());
    }
}
