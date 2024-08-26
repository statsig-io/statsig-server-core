use std::fs;
use std::path::PathBuf;
use serde_json::{Value, Map, };

pub fn enforce_array(value: &Value) -> Vec<Value> {
    value.as_array().unwrap().clone()
}

pub fn enforce_object(value: &Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

pub fn enforce_string(value: &Value) -> String {
    value.as_str().unwrap().to_string()
}

pub fn read_file_contents(path: &str) -> String {
    let mut base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base_path.push(path);

    fs::read_to_string(path).expect("Unable to read file")
}