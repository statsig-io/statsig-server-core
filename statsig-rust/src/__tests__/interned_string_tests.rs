use std::collections::HashMap;
use std::sync::Arc;

use more_asserts::assert_gt;
use serde_json::json;

use crate::interned_str;
use crate::interned_string::InternedString;

#[test]
fn test_interned_string() {
    let string = InternedString::from_str_ref("test");
    assert_eq!(string.as_str(), "test");
}

#[test]
fn test_interned_string_from_string() {
    let string = InternedString::from_string("test".to_string());
    assert_eq!(string.as_str(), "test");
}

#[test]
fn test_interned_string_from_bool() {
    let string = InternedString::from_bool(true);
    assert_eq!(string.as_str(), "true");
}

#[test]
fn test_interned_string_from_bool_macro() {
    let string = interned_str!(bool: true);
    assert_eq!(string.as_str(), "true");
}

#[test]
fn test_comparison_works() {
    let string = InternedString::from_str_ref("test");
    let string2 = InternedString::from_str_ref("test");
    assert_eq!(string, string2);
}

#[test]
fn test_comparison_works_with_different_strings() {
    let string = InternedString::from_str_ref("test");
    let string2 = InternedString::from_str_ref("test2");
    assert_ne!(string, string2);
}

#[test]
fn test_comparison_ignores_hash() {
    let string = InternedString {
        hash: 1,
        value: Arc::new("test".to_string()),
    };
    let string2 = InternedString {
        hash: 2,
        value: Arc::new("test".to_string()),
    };
    assert_eq!(string, string2);
}

#[test]
fn test_from_emoji() {
    let expo_str = InternedString::from_str_ref("👍");
    let str_ref = expo_str.as_str();

    assert_eq!(str_ref, "👍");
}

#[test]
fn test_from_non_english_string() {
    let expo_str = InternedString::from_str_ref("て");
    let str_ref = expo_str.as_str();

    assert_eq!(str_ref, "て");
}

#[test]
fn test_extracting_the_inner_value() {
    let expo_str = InternedString::from_str_ref("a_string");
    let owned_string = expo_str.unperformant_to_string();

    assert_eq!(owned_string, "a_string".to_string());
}

#[test]
fn test_jsonify_from_str_ref() {
    let map = HashMap::from([("key", InternedString::from_str_ref("value"))]);
    let serialized = serde_json::to_string(&map).unwrap();

    assert_eq!(serialized, r#"{"key":"value"}"#);
}

#[test]
fn test_jsonify_from_str_parts() {
    let map = HashMap::from([("key", InternedString::from_str_parts(&["value", "2"]))]);
    let serialized = json!(map).to_string();

    assert_eq!(serialized, r#"{"key":"value2"}"#);
}

#[test]
fn test_jsonify_two_ways() {
    let raw = r#"{"key":"value"}"#;

    let deserialized: HashMap<String, InternedString> = serde_json::from_str(raw).unwrap();

    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(deserialized.get("key").unwrap().as_str(), "value");
    assert_eq!(raw, serialized);
}

#[test]
fn test_jsonify_from_value() {
    let raw = json!(InternedString::from_str_ref("value"));
    let deserialized: InternedString = serde_json::from_value(raw).unwrap();

    assert_eq!(deserialized.as_str(), "value");
}

#[test]
fn test_consistency_across_initialization() {
    let from_str_ref = InternedString::from_str_ref("value2");
    let from_str_parts = InternedString::from_str_parts(&["value", "2"]);
    let from_json: InternedString = serde_json::from_str("\"value2\"").unwrap();

    assert_eq!(from_str_ref.hash, from_str_parts.hash);
    assert_eq!(from_str_parts.hash, from_json.hash);
}

#[test]
fn test_exposable_string_hash_value_is_consistent() {
    let s = "a_string";
    let mut hash = InternedString::from_str_ref(s).hash;

    for _ in 0..10000 {
        let new_hash = InternedString::from_str_ref(s).hash;
        assert_eq!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}

#[test]
fn test_exposable_string_hash_value_is_changing() {
    let mut hash = InternedString::from_str_ref("a_string").hash;

    for i in 0..10000 {
        let new_hash = InternedString::from_str_ref(format!("{i}").as_str()).hash;
        assert_ne!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}
