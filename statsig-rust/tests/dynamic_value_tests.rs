use std::collections::HashMap;

use more_asserts::assert_gt;
use serde_json::{json, Number, Value};
use statsig_rust::{dyn_value, evaluation::dynamic_string::DynamicString};

macro_rules! assert_dynamic_value {
    (
        $dynamic_value:expr,
        $expected_json:expr,
        null: $expected_null:expr,
        bool: $expected_bool:expr,
        int: $expected_int:expr,
        float: $expected_float:expr,
        timestamp: $expected_timestamp:expr,
        string: $expected_string:expr,
        array: $expected_array:expr,
        object: $expected_object:expr,
        ) => {
        assert_eq!($dynamic_value.null, $expected_null);
        assert_eq!($dynamic_value.bool_value, $expected_bool);
        assert_eq!($dynamic_value.int_value, $expected_int);
        assert_eq!($dynamic_value.float_value, $expected_float);
        assert_eq!($dynamic_value.timestamp_value, $expected_timestamp);
        assert_eq!($dynamic_value.string_value, $expected_string);
        assert_eq!($dynamic_value.array_value, $expected_array);
        assert_eq!($dynamic_value.object_value, $expected_object);
        assert_eq!($dynamic_value.json_value, $expected_json);

        assert_gt!($dynamic_value.hash_value, 0);
    };
}

#[test]
fn test_int_32_value() {
    assert_dynamic_value!(
        dyn_value!(1_i32),
        Value::Number(Number::from(1_i32)),
        null: None,
        bool: None,
        int: Some(1),
        float: Some(1.0),
        timestamp: None,
        string: Some(DynamicString::from("1".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_int_64_value() {
    assert_dynamic_value!(
        dyn_value!(1_i64),
        Value::Number(Number::from(1_i64)),
        null: None,
        bool: None,
        int: Some(1),
        float: Some(1.0),
        timestamp: None,
        string: Some(DynamicString::from("1".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_f64_with_precision_value() {
    assert_dynamic_value!(
        dyn_value!(1.23_f64),
        Value::Number(Number::from_f64(1.23).unwrap()),
        null: None,
        bool: None,
        int: None,
        float: Some(1.23),
        timestamp: None,
        string: Some(DynamicString::from("1.23".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_f64_with_no_precision_value() {
    assert_dynamic_value!(
        dyn_value!(1.0_f64),
        Value::Number(Number::from_f64(1.0).unwrap()),
        null: None,
        bool: None,
        int: None,
        float: Some(1.0),
        timestamp: None,
        // We drop the precision when its zero
        string: Some(DynamicString::from("1".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_value() {
    assert_dynamic_value!(
        dyn_value!("test"),
        Value::String("test".to_string()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        string: Some(DynamicString::from("test".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_float_value() {
    assert_dynamic_value!(
        dyn_value!("1.23"),
        Value::String("1.23".to_string()),
        null: None,
        bool: None,
        int: None,
        float: Some(1.23),
        timestamp: None,
        string: Some(DynamicString::from("1.23".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_int_value() {
    assert_dynamic_value!(
        dyn_value!("1"),
        Value::String("1".to_string()),
        null: None,
        bool: None,
        int: Some(1),
        float: Some(1.0),
        timestamp: Some(1),
        string: Some(DynamicString::from("1".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_rfc3339_date_value() {
    assert_dynamic_value!(
        dyn_value!("2021-01-01T00:00:00Z"),
        Value::String("2021-01-01T00:00:00Z".to_string()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: Some(1609459200000),
        string: Some(DynamicString::from("2021-01-01T00:00:00Z".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_native_date_value() {
    assert_dynamic_value!(
        dyn_value!("2015-09-05 23:56:04"),
        Value::String("2015-09-05 23:56:04".to_string()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: Some(1441497364000),
        string: Some(DynamicString::from("2015-09-05 23:56:04".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_bool_value() {
    assert_dynamic_value!(
        dyn_value!("true"),
        Value::String("true".to_string()),
        null: None,
        bool: None, // We don't parse bool values from strings
        int: None,
        float: None,
        timestamp: None,
        string: Some(DynamicString::from("true".to_string())),
        array: None,
        object: None,
    );
}

#[test]
fn test_string_json_object_value() {
    assert_dynamic_value!(
        dyn_value!("{\"test1\": \"test2\"}"),
        Value::String("{\"test1\": \"test2\"}".to_string()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        string: Some(DynamicString::from("{\"test1\": \"test2\"}".to_string())),
        array: None,
        object: None, // We don't parse JSON object strings into a map
    );
}

#[test]
fn test_string_json_array_value() {
    assert_dynamic_value!(
        dyn_value!("[\"test1\", \"test2\"]"),
        Value::String("[\"test1\", \"test2\"]".to_string()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        string: Some(DynamicString::from("[\"test1\", \"test2\"]".to_string())),
        array: None, // We don't parse JSON array strings into a vector
        object: None,
    );
}

#[test]
fn test_json_array_value() {
    assert_dynamic_value!(
        dyn_value!(serde_json::from_str::<Value>("[\"arr1\", \"arr2\"]").unwrap()),
        Value::Array(vec![Value::String("arr1".to_string()), Value::String("arr2".to_string())]),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        // Store a string for the "str_contains_any" | "str_contains_none" operators
        string: Some(DynamicString::from("[\"arr1\",\"arr2\"]".to_string())),
        array: Some(vec![dyn_value!("arr1"), dyn_value!("arr2")]),
        object: None,
    );
}

#[test]
fn test_json_array_with_mixed_types() {
    assert_dynamic_value!(
        dyn_value!(serde_json::from_str::<Value>("[\"a_string\", 1, true, 1.23]").unwrap()),
        Value::Array(vec![Value::String("a_string".to_string()), Value::Number(Number::from(1)), Value::Bool(true), Value::Number(Number::from_f64(1.23).unwrap())]),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        string: Some(DynamicString::from("[\"a_string\",1,true,1.23]".to_string())),
        array: Some(vec![dyn_value!("a_string"), dyn_value!(1), dyn_value!(true), dyn_value!(1.23)]),
        object: None,
    );
}

#[test]
fn test_json_object_value() {
    assert_dynamic_value!(
        dyn_value!(serde_json::from_str::<Value>("{\"object\": \"value\"}").unwrap()),
        Value::Object(json!({"object": "value"}).as_object().unwrap().clone()),
        null: None,
        bool: None,
        int: None,
        float: None,
        timestamp: None,
        string: None,
        array: None,
        object: Some(HashMap::from([("object".to_string(), dyn_value!("value"))])),
    );
}

#[test]
fn test_hash_value_is_consistent() {
    let map = json!({
        "employee_id": "1234567890",
        "is_active": true,
        "group_id": 87123471,
        "department": "engineering",
    });

    let mut hash = dyn_value!(&map).hash_value;
    for _ in 0..10000 {
        let value = dyn_value!(&map).hash_value;
        assert_eq!(hash, value);
        hash = value;
    }
}

#[test]
fn test_hash_value_is_changing() {
    let mut map = json!({
        "employee_id": "1234567890",
        "is_active": true,
        "group_id": 87123471,
        "department": "engineering",
    })
    .as_object()
    .unwrap()
    .clone();

    let mut hash = dyn_value!(&map).hash_value;
    for i in 0..10000 {
        map.insert("group_id".to_string(), json!(i));
        let value = dyn_value!(&map).hash_value;
        assert_ne!(hash, value);
        hash = value;
    }
}
