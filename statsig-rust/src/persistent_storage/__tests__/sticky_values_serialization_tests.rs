use std::collections::HashMap;

use crate::{
    interned_string::InternedString, specs_response::explicit_params::ExplicitParameters,
    DynamicReturnable, SecondaryExposure, StickyValues,
};

use serde_json::{json, Value};

#[test]
fn test_sticky_values_deserialization_from_value() {
    let raw_values = json!({
      "value": true,
      "json_value": {
        "header_text": "old user test",
      },
      "rule_id": "3ZCniK9rvnQyXDQlQ1tGD9",
      "group_name": "test_group",
      "secondary_exposures": [
        {
          "gate": "test_holdout",
          "gateValue": "true",
          "ruleID": "default",
        },
      ],
      "undelegated_secondary_exposures": [],
      "config_version": 1,
      "time": 1757974789243_u64,
      "config_delegate": null,
      "explicit_parameters": null,
    });

    let sticky = match serde_json::from_value::<StickyValues>(raw_values) {
        Ok(sticky) => sticky,
        Err(e) => {
            panic!("Failed to deserialize sticky values: {}", e);
        }
    };

    assert!(sticky.json_value.is_some());
}

#[test]
fn test_sticky_values_deserialization_from_str() {
    let raw_values = r#"{
      "value": true,
      "json_value": {
        "header_text": "old user test"
      },
      "rule_id": "3ZCniK9rvnQyXDQlQ1tGD9",
      "group_name": "test_group",
      "secondary_exposures": [
        {
          "gate": "test_holdout",
          "gateValue": "true",
          "ruleID": "default"
        }
      ],
      "undelegated_secondary_exposures": [],
      "config_version": 1,
      "time": 1757974789243,
      "config_delegate": null,
      "explicit_parameters": null
    }"#;

    let sticky = match serde_json::from_str::<StickyValues>(raw_values) {
        Ok(sticky) => sticky,
        Err(e) => {
            panic!("Failed to deserialize sticky values: {}", e);
        }
    };

    assert!(sticky.json_value.is_some());
}

#[test]
fn test_sticky_values_serialization_to_value() {
    let sticky = StickyValues {
        value: true,
        json_value: Some(DynamicReturnable::from_map(HashMap::new())),
        rule_id: Some(InternedString::from_str_ref("3ZCniK9rvnQyXDQlQ1tGD9")),
        group_name: Some(InternedString::from_str_ref("test_group")),
        secondary_exposures: vec![SecondaryExposure {
            gate: InternedString::from_str_ref("test_holdout"),
            gate_value: InternedString::from_str_ref("true"),
            rule_id: InternedString::default_rule_id(),
        }],
        undelegated_secondary_exposures: None,
        config_delegate: Some(InternedString::from_str_ref("test_delegate")),
        explicit_parameters: Some(ExplicitParameters::from_vec(vec!["test_param".to_string()])),
        time: Some(1757974789243_u64),
        config_version: Some(1),
    };

    let value = match serde_json::to_value(sticky) {
        Ok(value) => value,
        Err(e) => {
            panic!("Failed to serialize sticky values: {}", e);
        }
    };

    assert_eq!(value.get("value").unwrap(), &Value::Bool(true));
}

#[test]
fn test_sticky_values_serialization_to_str() {
    let sticky = StickyValues {
        value: true,
        json_value: Some(DynamicReturnable::from_map(HashMap::new())),
        rule_id: Some(InternedString::from_str_ref("3ZCniK9rvnQyXDQlQ1tGD9")),
        group_name: Some(InternedString::from_str_ref("test_group")),
        secondary_exposures: vec![SecondaryExposure {
            gate: InternedString::from_str_ref("test_holdout"),
            gate_value: InternedString::from_str_ref("true"),
            rule_id: InternedString::default_rule_id(),
        }],
        undelegated_secondary_exposures: None,
        config_delegate: Some(InternedString::from_str_ref("test_delegate")),
        explicit_parameters: Some(ExplicitParameters::from_vec(vec!["test_param".to_string()])),
        time: Some(1757974789243_u64),
        config_version: Some(1),
    };

    let value = match serde_json::to_string(&sticky) {
        Ok(value) => value,
        Err(e) => {
            panic!("Failed to serialize sticky values: {}", e);
        }
    };

    assert!(value.contains("test_delegate"));
}
