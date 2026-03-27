use std::collections::HashMap;

use assert_json_diff::assert_json_eq;
use serde_json::{json, Value as JsonValue};

use crate::evaluation::rkyv_value::{ArchivedRkyvValue, RkyvNumber, RkyvValue};

fn sample_json_value() -> JsonValue {
    json!({
        "null": null,
        "bool": true,
        "string": "value",
        "positive": 123,
        "negative": -45,
        "float": 3.25,
        "array": [
            null,
            false,
            7,
            -9,
            2.5,
            "nested",
            {
                "key": "value"
            }
        ],
        "object": {
            "inner_bool": false,
            "inner_array": [1, 2, 3]
        }
    })
}

#[test]
fn test_serde_deserializes_numbers_into_rkyv_number_variants() {
    match serde_json::from_value::<RkyvValue>(json!(42)).unwrap() {
        RkyvValue::Number(RkyvNumber::PosInt(42)) => {}
        value => panic!("expected PosInt(42), got {:?}", value),
    }

    match serde_json::from_value::<RkyvValue>(json!(9223372036854775808u64)).unwrap() {
        RkyvValue::Number(RkyvNumber::PosInt(9223372036854775808)) => {}
        value => panic!("expected PosInt(9223372036854775808), got {:?}", value),
    }

    match serde_json::from_value::<RkyvValue>(json!(-42)).unwrap() {
        RkyvValue::Number(RkyvNumber::NegInt(-42)) => {}
        value => panic!("expected NegInt(-42), got {:?}", value),
    }

    match serde_json::from_value::<RkyvValue>(json!(3.25)).unwrap() {
        RkyvValue::Number(RkyvNumber::Float(3.25)) => {}
        value => panic!("expected Float(3.25), got {:?}", value),
    }
}

#[test]
fn test_rkyv_value_round_trips_nested_json_through_serde() {
    let original = sample_json_value();
    let value = serde_json::from_value::<RkyvValue>(original.clone()).unwrap();

    assert_json_eq!(serde_json::to_value(&value).unwrap(), original);
}

#[test]
fn test_hash_map_of_rkyv_values_round_trips_through_serde() {
    let original = sample_json_value();
    let converted = serde_json::from_value::<HashMap<String, RkyvValue>>(original.clone()).unwrap();

    assert!(matches!(
        converted.get("positive"),
        Some(RkyvValue::Number(RkyvNumber::PosInt(123)))
    ));
    assert!(matches!(
        converted.get("negative"),
        Some(RkyvValue::Number(RkyvNumber::NegInt(-45)))
    ));
    assert!(matches!(
        converted.get("float"),
        Some(RkyvValue::Number(RkyvNumber::Float(3.25)))
    ));

    let round_trip = JsonValue::Object(
        converted
            .into_iter()
            .map(|(key, value)| (key, serde_json::to_value(value).unwrap()))
            .collect(),
    );

    assert_json_eq!(round_trip, original);
}

#[test]
fn test_archived_rkyv_value_serializes_like_json() {
    let original = sample_json_value();
    let value = serde_json::from_value::<RkyvValue>(original.clone()).unwrap();

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&value).unwrap();
    let archived = rkyv::access::<ArchivedRkyvValue, rkyv::rancor::Error>(&bytes).unwrap();

    assert_json_eq!(serde_json::to_value(archived).unwrap(), original.clone());

    let deserialized = rkyv::deserialize::<RkyvValue, rkyv::rancor::Error>(archived).unwrap();
    assert_json_eq!(serde_json::to_value(&deserialized).unwrap(), original);
}
