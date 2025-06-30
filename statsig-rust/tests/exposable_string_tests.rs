use std::collections::HashMap;

use more_asserts::assert_gt;
use serde_json::json;
use statsig_rust::event_logging::exposable_string::ExposableString;

#[test]
fn test_as_str() {
    let expo_str = ExposableString::from_str_ref("a_string");
    let str_ref = expo_str.as_str();

    assert_eq!(str_ref, "a_string");
}

#[test]
fn test_from_emoji() {
    let expo_str = ExposableString::from_str_ref("👍");
    let str_ref = expo_str.as_str();

    assert_eq!(str_ref, "👍");
}

#[test]
fn test_from_non_english_string() {
    let expo_str = ExposableString::from_str_ref("て");
    let str_ref = expo_str.as_str();

    assert_eq!(str_ref, "て");
}

#[test]
fn test_extracting_the_inner_value() {
    let expo_str = ExposableString::from_str_ref("a_string");
    let owned_string = expo_str.unperformant_to_string();

    assert_eq!(owned_string, "a_string".to_string());
}

#[test]
fn test_jsonify_from_str_ref() {
    let map = HashMap::from([("key", ExposableString::from_str_ref("value"))]);
    let serialized = serde_json::to_string(&map).unwrap();

    assert_eq!(serialized, r#"{"key":"value"}"#);
}

#[test]
fn test_jsonify_from_str_parts() {
    let map = HashMap::from([("key", ExposableString::from_str_parts(&["value", "2"]))]);
    let serialized = json!(map).to_string();

    assert_eq!(serialized, r#"{"key":"value2"}"#);
}

#[test]
fn test_jsonify_two_ways() {
    let raw = r#"{"key":"value"}"#;

    let deserialized: HashMap<String, ExposableString> = serde_json::from_str(raw).unwrap();

    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(deserialized.get("key").unwrap().as_str(), "value");
    assert_eq!(raw, serialized);
}

#[test]
fn test_jsonify_from_value() {
    let raw = json!(ExposableString::from_str_ref("value"));
    let deserialized: ExposableString = serde_json::from_value(raw).unwrap();

    assert_eq!(deserialized.as_str(), "value");
}

#[test]
fn test_consistency_across_initialization() {
    let from_str_ref = ExposableString::from_str_ref("value2");
    let from_str_parts = ExposableString::from_str_parts(&["value", "2"]);
    let from_json: ExposableString = serde_json::from_str("\"value2\"").unwrap();

    assert_eq!(from_str_ref.hash_value, from_str_parts.hash_value);
    assert_eq!(from_str_parts.hash_value, from_json.hash_value);
}

#[test]
fn test_exposable_string_hash_value_is_consistent() {
    let s = "a_string";
    let mut hash = ExposableString::from_str_ref(s).hash_value;

    for _ in 0..10000 {
        let new_hash = ExposableString::from_str_ref(s).hash_value;
        assert_eq!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}

#[test]
fn test_exposable_string_hash_value_is_changing() {
    let mut hash = ExposableString::from_str_ref("a_string").hash_value;

    for i in 0..10000 {
        let new_hash = ExposableString::from_str_ref(format!("{i}").as_str()).hash_value;
        assert_ne!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}
