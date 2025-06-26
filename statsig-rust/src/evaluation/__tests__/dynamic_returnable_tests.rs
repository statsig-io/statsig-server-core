use serde_json::json;

use crate::evaluation::dynamic_returnable::{self, DynamicReturnable};
use std::{collections::HashMap, sync::Mutex};

lazy_static::lazy_static! {
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

#[test]
fn test_jsonify_object_two_ways() {
    let _lock = TEST_LOCK.lock().unwrap();

    let raw = r#"{"test":{"key":"value"}}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();
    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(raw, serialized);
}

#[test]
fn test_jsonify_bool_two_ways() {
    let _lock = TEST_LOCK.lock().unwrap();
    let raw = r#"{"test":true}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();
    let serialized = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(raw, serialized);
}

#[test]
fn test_memoization_from_json_object() {
    let _lock = TEST_LOCK.lock().unwrap();
    let raw = r#"{"once":{"key":"value"},"twice":{"key":"value"}}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();

    assert_eq!(get_memo_len(), 1);

    drop(deserialized);

    assert_eq!(get_memo_len(), 0);
}

#[test]
fn test_memoization_from_json_bool() {
    let _lock = TEST_LOCK.lock().unwrap();
    let raw = r#"{"once":true,"twice":true}"#;

    let deserialized: HashMap<String, DynamicReturnable> = serde_json::from_str(raw).unwrap();

    assert_eq!(get_memo_len(), 1);

    drop(deserialized);

    assert_eq!(get_memo_len(), 0);
}

#[test]
fn test_memoization_from_map() {
    let _lock = TEST_LOCK.lock().unwrap();

    let value = HashMap::from([("key".to_string(), json!("value"))]);
    let dyn_returnable = DynamicReturnable::from_map(value);

    assert_eq!(get_memo_len(), 1);

    drop(dyn_returnable);

    assert_eq!(get_memo_len(), 0);
}

fn get_memo_len() -> usize {
    let memo = dynamic_returnable::MEMOIZED_VALUES.lock().unwrap();

    for (key, value) in memo.iter() {
        let value = value.upgrade().unwrap();
        println!(
            "{}: RawValue({:?}), Bool({:?}), Json({:?})",
            key, value.raw_value, value.bool_value, value.json_value
        );
    }

    memo.len()
}
