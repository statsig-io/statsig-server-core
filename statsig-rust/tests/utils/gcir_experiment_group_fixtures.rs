use std::collections::HashMap;

use serde_json::{json, Value};
use statsig_rust::{
    interned_string::InternedString,
    specs_response::{
        spec_types::{Condition, Spec, SpecsResponseFull},
        specs_hash_map::SpecPointer,
    },
    StatsigUser,
};

pub fn add_condition(specs: &mut SpecsResponseFull, key: &str, value: Value) {
    specs.condition_map.insert(
        InternedString::from_str_ref(key),
        serde_json::from_value::<Condition>(value).expect("condition should deserialize"),
    );
}

pub fn add_config(specs: &mut SpecsResponseFull, name: &str, rules: Value) {
    let spec = serde_json::from_value::<Spec>(json!({
        "type": "dynamic_config",
        "salt": name,
        "enabled": true,
        "defaultValue": {},
        "rules": rules,
        "idType": "userID",
        "entity": "experiment",
        "version": 1,
        "isActive": true,
        "hasSharedParams": false
    }))
    .expect("config should deserialize");
    specs.dynamic_configs.insert(
        InternedString::from_str_ref(name),
        SpecPointer::from_spec(spec),
    );
}

pub fn add_layer(specs: &mut SpecsResponseFull, name: &str, rules: Value) {
    let spec = serde_json::from_value::<Spec>(json!({
        "type": "dynamic_config",
        "salt": name,
        "enabled": true,
        "defaultValue": { "default": "value" },
        "rules": rules,
        "idType": "userID",
        "entity": "layer",
        "version": 1,
        "isActive": true,
        "hasSharedParams": false,
        "useNewLayerEval": true
    }))
    .expect("layer should deserialize");
    specs.layer_configs.insert(
        InternedString::from_str_ref(name),
        SpecPointer::from_spec(spec),
    );
}

pub fn add_gate(specs: &mut SpecsResponseFull, name: &str, entity: &str, rules: Value) {
    let spec = serde_json::from_value::<Spec>(json!({
        "type": "feature_gate",
        "salt": name,
        "enabled": true,
        "defaultValue": false,
        "rules": rules,
        "idType": "profileId",
        "entity": entity,
        "version": 1
    }))
    .expect("gate should deserialize");
    specs.feature_gates.insert(
        InternedString::from_str_ref(name),
        SpecPointer::from_spec(spec),
    );
}

pub fn rule(id: &str, conditions: &[&str], value: Value, group: Option<&str>) -> Value {
    let mut rule = json!({
        "name": id,
        "passPercentage": 100.0,
        "conditions": conditions,
        "returnValue": value,
        "id": id,
        "salt": id,
        "idType": "userID"
    });
    if let Some(group) = group {
        rule["groupName"] = json!(group);
        rule["isExperimentGroup"] = json!(true);
    }
    rule
}

pub fn unit_condition(values: &[&str], id_type: &str) -> Value {
    json!({
        "type": "unit_id",
        "targetValue": values,
        "operator": "any",
        "field": null,
        "additionalValues": {},
        "idType": id_type
    })
}

pub fn gate_condition(kind: &str, gate: &str) -> Value {
    json!({
        "type": kind,
        "targetValue": gate,
        "operator": null,
        "field": null,
        "additionalValues": {},
        "idType": "profileId"
    })
}

pub fn experiment_group_condition(experiment: &str, group: &str) -> Value {
    json!({
        "type": "experiment_group",
        "targetValue": [group],
        "operator": "any",
        "field": experiment,
        "additionalValues": { "experiment_name": experiment },
        "idType": "profileId"
    })
}

pub fn add_target_experiment(specs: &mut SpecsResponseFull, name: &str, segment: &str) {
    let override_condition = format!("{name}-override-condition");
    let targeting_condition = format!("{name}-targeting-condition");
    let group_condition = format!("{name}-group-condition");
    add_condition(
        specs,
        &override_condition,
        unit_condition(&["profile-forced"], "profileId"),
    );
    add_condition(
        specs,
        &targeting_condition,
        gate_condition("fail_gate", segment),
    );
    add_condition(
        specs,
        &group_condition,
        unit_condition(&["eligible"], "userID"),
    );
    let mut override_rule = rule(
        &format!("{name}-treatment-rule:profileId:id_override"),
        &[&override_condition],
        json!({ "payload": "forced-treatment" }),
        None,
    );
    override_rule["groupName"] = json!("treatment");
    add_config(
        specs,
        name,
        json!([
            override_rule,
            rule(
                "inlineTargetingRules",
                &[&targeting_condition],
                json!({}),
                None
            ),
            rule(
                &format!("{name}-treatment-rule"),
                &[&group_condition],
                json!({ "payload": "treatment" }),
                Some("treatment")
            )
        ]),
    );
}

pub fn experiment_group_specs() -> SpecsResponseFull {
    let mut specs = SpecsResponseFull {
        has_updates: true,
        time: 1,
        ..Default::default()
    };

    add_condition(
        &mut specs,
        "reference-control",
        unit_condition(&["profile-control"], "profileId"),
    );
    add_condition(
        &mut specs,
        "reference-test",
        unit_condition(&["profile-test"], "profileId"),
    );
    add_config(
        &mut specs,
        "reference-experiment",
        json!([
            rule(
                "reference-control-rule",
                &["reference-control"],
                json!({ "reference": "control" }),
                Some("control")
            ),
            rule(
                "reference-test-rule",
                &["reference-test"],
                json!({ "reference": "test" }),
                Some("test")
            )
        ]),
    );

    add_condition(
        &mut specs,
        "health-condition",
        unit_condition(&["profile-control"], "profileId"),
    );
    add_gate(
        &mut specs,
        "health-gate",
        "feature_gate",
        json!([rule(
            "health-rule",
            &["health-condition"],
            json!(true),
            None
        )]),
    );

    add_condition(
        &mut specs,
        "group-only-condition",
        experiment_group_condition("reference-experiment", "control"),
    );
    add_gate(
        &mut specs,
        "segment:inline_targeting:group-only",
        "segment",
        json!([rule(
            "group-only-segment-rule",
            &["group-only-condition"],
            json!(true),
            None
        )]),
    );

    add_condition(
        &mut specs,
        "combined-health",
        gate_condition("pass_gate", "health-gate"),
    );
    add_condition(
        &mut specs,
        "combined-group",
        experiment_group_condition("reference-experiment", "control"),
    );
    add_gate(
        &mut specs,
        "segment:inline_targeting:combined",
        "segment",
        json!([rule(
            "combined-segment-rule",
            &["combined-health", "combined-group"],
            json!(true),
            None
        )]),
    );

    add_condition(
        &mut specs,
        "gate-only-health",
        gate_condition("pass_gate", "health-gate"),
    );
    add_gate(
        &mut specs,
        "segment:inline_targeting:gate-only",
        "segment",
        json!([rule(
            "gate-only-segment-rule",
            &["gate-only-health"],
            json!(true),
            None
        )]),
    );

    add_target_experiment(
        &mut specs,
        "group-only",
        "segment:inline_targeting:group-only",
    );
    add_target_experiment(&mut specs, "combined", "segment:inline_targeting:combined");
    add_target_experiment(
        &mut specs,
        "gate-only",
        "segment:inline_targeting:gate-only",
    );

    specs
}

pub fn test_user(user_id: &str, profile_id: &str) -> StatsigUser {
    let mut user = StatsigUser::with_user_id(user_id);
    user.set_custom_ids(HashMap::from([("profileId", profile_id)]));
    user
}
