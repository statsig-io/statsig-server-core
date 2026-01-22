use serde_json::json;

use crate::{evaluation::dynamic_returnable::DynamicReturnable, interned_values::InternedStore};
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial]
fn test_jsonify_object_two_ways() {
    let raw = r#"{"test":{"key":"value"}}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();
    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(raw, serialized);
}

#[test]
#[serial]
fn test_jsonify_bool_two_ways() {
    let raw = r#"{"test":true}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();
    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(raw, serialized);
}

#[test]
#[serial]
fn test_memoization_from_json_object() {
    let raw = r#"{"once":{"key":"value"},"twice":{"key":"value"}}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();

    assert_eq!(get_memo_len(), 1);

    drop(deserialized);

    assert_eq!(get_memo_len(), 0);
}

#[test]
#[serial]
fn test_memoization_from_json_bool() {
    let raw = r#"{"once":true,"twice":true}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();
    let value = deserialized.get("once").unwrap();
    assert_eq!(value.get_bool(), Some(true));

    assert_eq!(get_memo_len(), 0, "Bools are not memoized");
}

#[test]
#[serial]
fn test_memoization_from_map() {
    let value = HashMap::from([("key".to_string(), json!("value"))]);

    let dyn_returnable = DynamicReturnable::from_map(value.clone());
    assert_eq!(get_memo_len(), 1);

    let dyn_returnable2 = DynamicReturnable::from_map(value);
    assert_eq!(get_memo_len(), 1);

    drop(dyn_returnable);
    assert_eq!(get_memo_len(), 1);

    drop(dyn_returnable2);
    assert_eq!(get_memo_len(), 0);
}

fn get_memo_len() -> usize {
    InternedStore::get_memoized_len().1
}
