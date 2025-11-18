mod utils;

use assert_json_diff::{assert_json_matches, CompareMode, Config, NumericMode};
use serde_json::Value;
use statsig_rust::specs_response::spec_types::SpecsResponseFull;

use crate::utils::helpers::load_contents;

#[test]
fn test_full_response_serialization_from_str() {
    let raw_dcs = load_contents("eval_proj_dcs.json");

    let specs_response = serde_json::from_str::<SpecsResponseFull>(&raw_dcs).unwrap();

    assert!(specs_response.has_updates)
}

#[test]
fn test_full_response_serialization_from_value() {
    let raw_dcs_str = load_contents("eval_proj_dcs.json");
    let raw_dcs = serde_json::from_str::<Value>(&raw_dcs_str).unwrap();

    let specs_response = serde_json::from_value::<SpecsResponseFull>(raw_dcs).unwrap();

    assert!(specs_response.has_updates)
}

#[test]
fn test_json_roundtrip() {
    let mut raw_dcs = load_contents("eval_proj_dcs.json");

    // The param store has a name for the param that is pointless
    raw_dcs = raw_dcs.replace(r#""name":"bool_param","#, "");

    let specs_response = serde_json::from_str::<SpecsResponseFull>(&raw_dcs).unwrap();

    let specs_json = serde_json::to_string_pretty(&specs_response).unwrap();

    let original: Value = serde_json::from_str(&raw_dcs).unwrap();
    let roundtrip: Value = serde_json::from_str(&specs_json).unwrap();

    verify_null_preservation(&original, &roundtrip);

    let processed_original = remove_null_values(&original);
    let processed_roundtrip = remove_null_values(&roundtrip);

    let config = Config::new(CompareMode::Strict).numeric_mode(NumericMode::AssumeFloat);

    let orig_feature_gates = processed_original.get("feature_gates").unwrap();
    let round_feature_gates = processed_roundtrip.get("feature_gates").unwrap();
    assert_json_matches!(orig_feature_gates, round_feature_gates, config.clone());

    let orig_dynamic_configs = processed_original.get("dynamic_configs").unwrap();
    let round_dynamic_configs = processed_roundtrip.get("dynamic_configs").unwrap();
    assert_json_matches!(orig_dynamic_configs, round_dynamic_configs, config.clone());

    let orig_layer_configs = processed_original.get("layer_configs").unwrap();
    let round_layer_configs = processed_roundtrip.get("layer_configs").unwrap();
    assert_json_matches!(orig_layer_configs, round_layer_configs, config.clone());

    let orig_condition_map = processed_original.get("condition_map").unwrap();
    let round_condition_map = processed_roundtrip.get("condition_map").unwrap();
    assert_json_matches!(orig_condition_map, round_condition_map, config.clone());

    let mut remaining_original = processed_original.clone();
    let mut remaining_roundtrip = processed_roundtrip.clone();

    for field in &[
        "feature_gates",
        "dynamic_configs",
        "layer_configs",
        "condition_map",
    ] {
        if let Some(obj) = remaining_original.as_object_mut() {
            obj.remove(*field);
        }
        if let Some(obj) = remaining_roundtrip.as_object_mut() {
            obj.remove(*field);
        }
    }

    assert_json_matches!(remaining_original, remaining_roundtrip, config);
}

fn verify_null_preservation(original: &Value, roundtrip: &Value) {
    let null_fields = ["field", "operator", "targetValue", "additionalValues"];

    let orig_condition_map = match original.get("condition_map") {
        Some(Value::Object(map)) => map,
        _ => return,
    };

    let round_condition_map = match roundtrip.get("condition_map") {
        Some(Value::Object(map)) => map,
        _ => return,
    };

    for (condition_id, orig_condition) in orig_condition_map {
        let orig_condition_obj = match orig_condition {
            Value::Object(obj) => obj,
            _ => continue,
        };

        let round_condition = match round_condition_map.get(condition_id) {
            Some(condition) => condition,
            None => panic!("Condition {condition_id} missing in roundtrip"),
        };

        let round_condition_obj = match round_condition {
            Value::Object(obj) => obj,
            _ => continue,
        };

        for &field_name in &null_fields {
            if let Some(field_value) = orig_condition_obj.get(field_name) {
                if field_value.is_null() {
                    let round_field_value = match round_condition_obj.get(field_name) {
                        Some(value) => value,
                        None => panic!("'{field_name}' missing in condition {condition_id}"),
                    };

                    assert!(
                        round_field_value.is_null(),
                        "{field_name} should be null in condition {condition_id}"
                    );
                }
            }
        }
    }
}

fn remove_null_values(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                if !v.is_null() {
                    new_map.insert(k.clone(), remove_null_values(v));
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let new_arr = arr
                .iter()
                .filter(|v| !v.is_null())
                .map(remove_null_values)
                .collect();
            Value::Array(new_arr)
        }
        _ => value.clone(),
    }
}
