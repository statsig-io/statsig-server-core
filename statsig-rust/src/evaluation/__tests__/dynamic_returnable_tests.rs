use serde_json::json;

use crate::evaluation::dynamic_returnable::{self, DynamicReturnable};
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

    assert_eq!(get_memo_len(), 1);

    drop(deserialized);

    assert_eq!(get_memo_len(), 0);
}

#[test]
#[serial]
fn test_memoization_from_map() {
    let value = HashMap::from([("key".to_string(), json!("value"))]);
    let dyn_returnable = DynamicReturnable::from_map(value);

    assert_eq!(get_memo_len(), 1);

    drop(dyn_returnable);

    assert_eq!(get_memo_len(), 0);
}

fn get_memo_len() -> usize {
    let memo = dynamic_returnable::MEMOIZED_VALUES.try_lock().unwrap();

    for (key, value) in memo.iter() {
        let value = value.upgrade().unwrap();
        println!("{}: RawValue({:?})", key, value.raw_value);
    }

    memo.len()
}
