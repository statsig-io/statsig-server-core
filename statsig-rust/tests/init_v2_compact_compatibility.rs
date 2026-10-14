use serde::Deserialize;
use serde_json::json;
use statsig_rust::{
    evaluation::evaluation_types_initialize_v2::{
        BaseEvaluationInitV2, ExperimentEvaluationInitV2,
    },
    gcir::gcir_formatter::GCIRHashable,
    interned_string::InternedString,
};

#[derive(Deserialize)]
struct LegacyExperimentEvaluationInitV2 {
    #[serde(flatten)]
    base: BaseEvaluationInitV2,
    #[serde(rename = "i")]
    id_type: Option<InternedString>,
    #[serde(rename = "v")]
    value: InternedString,
    #[serde(rename = "gn")]
    group_name: Option<InternedString>,
    #[serde(rename = "ea")]
    is_experiment_active: Option<bool>,
    #[serde(rename = "ue")]
    is_user_in_experiment: Option<bool>,
}

#[test]
fn old_public_experiment_literal_still_compiles() {
    let _ = ExperimentEvaluationInitV2 {
        base: BaseEvaluationInitV2 {
            rule_id: None,
            secondary_exposures: None,
        },
        id_type: None,
        value: InternedString::from_str_ref("value"),
        group_name: None,
        is_experiment_active: None,
        is_user_in_experiment: None,
    };
}

#[test]
fn legacy_compact_experiment_hash_is_stable() {
    let evaluation = ExperimentEvaluationInitV2 {
        base: BaseEvaluationInitV2 {
            rule_id: Some(InternedString::from_str_ref("rule")),
            secondary_exposures: None,
        },
        id_type: Some(InternedString::from_str_ref("userID")),
        value: InternedString::from_str_ref("value"),
        group_name: Some(InternedString::from_str_ref("group")),
        is_experiment_active: Some(true),
        is_user_in_experiment: Some(false),
    };

    assert_eq!(
        evaluation.create_hash(&InternedString::from_str_ref("experiment")),
        17358448780810560084
    );
}

#[test]
fn old_compact_payload_deserializes_with_the_current_public_type() {
    let old_payload = json!({"r": "rule", "i": "userID", "v": "value"});

    let decoded: ExperimentEvaluationInitV2 = serde_json::from_value(old_payload).unwrap();

    assert_eq!(decoded.value.as_str(), "value");
    assert_eq!(decoded.base.rule_id.unwrap().as_str(), "rule");
}

#[test]
fn new_compact_payload_deserializes_with_a_legacy_serde_decoder() {
    let new_payload = json!({
        "r": "rule",
        "i": "userID",
        "v": "value",
        "ep": ["param"],
        "gn": "group",
        "ea": true,
        "ue": false,
    });

    let decoded: LegacyExperimentEvaluationInitV2 = serde_json::from_value(new_payload).unwrap();

    assert_eq!(decoded.value.as_str(), "value");
    assert_eq!(decoded.base.rule_id.unwrap().as_str(), "rule");
    assert_eq!(decoded.id_type.unwrap().as_str(), "userID");
    assert_eq!(decoded.group_name.unwrap().as_str(), "group");
    assert_eq!(decoded.is_experiment_active, Some(true));
    assert_eq!(decoded.is_user_in_experiment, Some(false));
}
