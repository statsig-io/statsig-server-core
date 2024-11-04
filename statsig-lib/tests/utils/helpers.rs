use serde_json::{Map, Value};

pub fn enforce_array(value: &Value) -> Vec<Value> {
    value.as_array().unwrap().clone()
}

pub fn enforce_object(value: &Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

pub fn enforce_string(value: &Value) -> String {
    value.as_str().unwrap().to_string()
}
