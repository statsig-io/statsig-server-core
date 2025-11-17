use std::collections::HashMap;

use assert_json_diff::{CompareMode, Config, NumericMode};
use serde_json::{json, Value};

use crate::{
    interned_string::InternedString,
    networking::ResponseData,
    specs_response::{proto_specs::deserialize_protobuf, spec_types::SpecsResponseFull},
};

const PROTO_SPECS_BYTES: &[u8] = include_bytes!("test-dcs-v2.pb.br");
const JSON_SPECS_BYTES: &[u8] = include_bytes!("test-dcs-v2.json");

#[test]
fn test_all_specs_are_present() {
    let specs = get_deserialized_specs();

    assert_eq!(specs.feature_gates.len(), 82);
    assert_eq!(specs.dynamic_configs.len(), 63);
    assert_eq!(specs.layer_configs.len(), 12);
}

#[test]
fn test_top_level_json_vs_proto() {
    let mut proto_specs = get_deserialized_specs();

    proto_specs.feature_gates.clear();
    proto_specs.dynamic_configs.clear();
    proto_specs.layer_configs.clear();

    let proto_specs_string = serde_json::to_string(&proto_specs).unwrap();
    let mut proto_specs_val: serde_json::Map<String, Value> =
        serde_json::from_str(&proto_specs_string).unwrap();

    let mut json_string = String::from_utf8(JSON_SPECS_BYTES.to_vec()).unwrap();

    // proto defaults to null for empty additionalValues
    json_string = json_string.replace(r#""additionalValues":{}"#, r#""additionalValues":null"#);

    // proto doesn't include name field, its implicit
    json_string = json_string.replace(r#""name":"bool_param","#, r#""#);

    let mut json_specs: serde_json::Map<String, Value> =
        serde_json::from_str(&json_string).unwrap();

    // These are compared in the individual tests
    json_specs.insert("feature_gates".to_string(), json!({}));
    json_specs.insert("dynamic_configs".to_string(), json!({}));
    json_specs.insert("layer_configs".to_string(), json!({}));

    normalize_null_condition_fields(&mut json_specs);
    normalize_null_condition_fields(&mut proto_specs_val);

    assert_json_diff::assert_json_matches!(proto_specs_val, json_specs, get_json_compare_config());
}

#[test]
fn test_a_gate_json_vs_proto() {
    let mut proto_spec = get_proto_spec("feature_gates", "test_public");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("feature_gates", "test_public").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_a_gate_json_vs_proto_resynced() {
    let mut proto_spec = get_proto_spec_resynced("feature_gates", "test_public");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("feature_gates", "test_public");

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_big_gate_json_vs_proto() {
    let mut proto_spec = get_proto_spec("feature_gates", "test_user_id_and_custom_field");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("feature_gates", "test_user_id_and_custom_field").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_big_gate_json_vs_proto_resynced() {
    let mut proto_spec = get_proto_spec_resynced("feature_gates", "test_user_id_and_custom_field");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("feature_gates", "test_user_id_and_custom_field").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_a_dynamic_config_json_vs_proto() {
    let mut proto_spec = get_proto_spec("dynamic_configs", "test_email_config");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("dynamic_configs", "test_email_config").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_a_dynamic_config_json_vs_proto_resynced() {
    let mut proto_spec = get_proto_spec_resynced("dynamic_configs", "test_email_config");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("dynamic_configs", "test_email_config").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_experiment_json_vs_proto() {
    let mut proto_spec = get_proto_spec("dynamic_configs", "experiment_with_many_params");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("dynamic_configs", "experiment_with_many_params").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_experiment_json_vs_proto_resynced() {
    let mut proto_spec = get_proto_spec_resynced("dynamic_configs", "experiment_with_many_params");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("dynamic_configs", "experiment_with_many_params").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_a_layer_json_vs_proto() {
    let mut proto_spec = get_proto_spec("layer_configs", "layer_with_many_params");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("layer_configs", "layer_with_many_params").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

#[test]
fn test_a_layer_json_vs_proto_resynced() {
    let mut proto_spec = get_proto_spec("layer_configs", "layer_with_many_params");
    proto_spec.remove("checksum");

    let json_spec = get_json_spec("layer_configs", "layer_with_many_params").clone();

    assert_json_diff::assert_json_matches!(proto_spec, json_spec, get_json_compare_config());
}

fn deserialize_from(current_specs: &mut SpecsResponseFull, next_specs: &mut SpecsResponseFull) {
    let bytes = PROTO_SPECS_BYTES.to_vec();
    let mut data = ResponseData::from_bytes(bytes);
    deserialize_protobuf(current_specs, next_specs, &mut data).unwrap();
}

fn get_deserialized_specs() -> SpecsResponseFull {
    let mut current_specs = SpecsResponseFull::default();
    let mut next_specs = SpecsResponseFull::default();
    deserialize_from(&mut current_specs, &mut next_specs);
    next_specs
}

fn get_proto_spec(top_level: &str, name: &str) -> serde_json::Map<String, Value> {
    let name = InternedString::from_str_ref(name);
    let mut proto_specs = get_deserialized_specs();
    let ptr = match top_level {
        "feature_gates" => proto_specs.feature_gates.remove(&name).unwrap(),
        "dynamic_configs" => proto_specs.dynamic_configs.remove(&name).unwrap(),
        "layer_configs" => proto_specs.layer_configs.remove(&name).unwrap(),
        _ => panic!("Invalid top level: {}", top_level),
    };

    match serde_json::to_value(ptr).expect("Failed to convert SpecPointer to Value") {
        Value::Object(obj) => obj,
        _ => panic!("Expected object"),
    }
}

/// Same as get_proto_spec, but passes the response back through the deserializer to mimic syncing.
fn get_proto_spec_resynced(top_level: &str, name: &str) -> serde_json::Map<String, Value> {
    let mut specs_one = SpecsResponseFull::default();
    let mut specs_two = SpecsResponseFull::default();
    deserialize_from(&mut specs_one, &mut specs_two);
    deserialize_from(&mut specs_two, &mut specs_one);

    let name = InternedString::from_str_ref(name);
    let ptr = match top_level {
        "feature_gates" => specs_one.feature_gates.remove(&name).unwrap(),
        "dynamic_configs" => specs_one.dynamic_configs.remove(&name).unwrap(),
        "layer_configs" => specs_one.layer_configs.remove(&name).unwrap(),
        _ => panic!("Invalid top level: {}", top_level),
    };

    match serde_json::to_value(ptr).expect("Failed to convert SpecPointer to Value") {
        Value::Object(obj) => obj,
        _ => panic!("Expected object"),
    }
}

fn get_json_spec(top_level: &str, name: &str) -> Value {
    let json_specs: HashMap<String, Value> = serde_json::from_slice(JSON_SPECS_BYTES).unwrap();
    json_specs
        .get(top_level)
        .unwrap()
        .get(name)
        .unwrap()
        .clone()
}

fn get_json_compare_config() -> Config {
    Config::new(CompareMode::Strict).numeric_mode(NumericMode::AssumeFloat)
}

/// Proto skips null 'field' fields in the condition map. JSON may or may not include them.
fn normalize_null_condition_fields(value: &mut serde_json::Map<String, Value>) {
    let condition_map = value.get_mut("condition_map").unwrap();
    let condition_map_obj = condition_map.as_object_mut().unwrap();
    for (_, v) in condition_map_obj {
        let is_null = match v.get_mut("field") {
            Some(field) => field.is_null(),
            None => continue,
        };

        if is_null {
            v.as_object_mut().unwrap().remove("field");
        }
    }
}
