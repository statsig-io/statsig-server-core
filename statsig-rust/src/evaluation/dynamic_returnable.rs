use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{value::RawValue, Value as JsonValue};

use crate::{hashing::djb2, log_e};

const TAG: &str = "DynamicReturnable";

lazy_static::lazy_static! {
    static ref MEMOIZED_VALUES: Mutex<HashMap<String, Weak<MemoizedValue>>> =
        Mutex::new(HashMap::new());
}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicReturnable {
    hash: String,
    value: Arc<MemoizedValue>,
}

impl DynamicReturnable {
    pub fn get_bool(&self) -> Option<bool> {
        self.value.bool_value
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        self.value.json_value.clone()
    }

    fn new(hash: String, value: Arc<MemoizedValue>) -> Self {
        Self { hash, value }
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value: &'de RawValue = Deserialize::deserialize(deserializer)?;
        let raw_value_str = raw_value.get();
        let hash = djb2(raw_value_str);

        if let Some(value) = get_memoized_value(&hash) {
            return Ok(DynamicReturnable { hash, value });
        }

        let value = MemoizedValue::new(raw_value_str);
        let weak_value = Arc::downgrade(&value);

        let new_returnable = DynamicReturnable::new(hash.clone(), value);
        set_memoized_value(&hash, weak_value);

        Ok(new_returnable)
    }
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(bool_value) = self.value.bool_value {
            return bool_value.serialize(serializer);
        }

        if let Some(json_value) = &self.value.json_value {
            return json_value.serialize(serializer);
        }

        JsonValue::Null.serialize(serializer)
    }
}

impl Drop for DynamicReturnable {
    fn drop(&mut self) {
        let mut memo = match MEMOIZED_VALUES.lock() {
            Ok(values) => values,
            Err(e) => {
                log_e!(TAG, "Failed to lock memoized values: {}", e);
                return;
            }
        };

        let found = match memo.get(&self.hash) {
            Some(value) => value,
            None => return,
        };

        if found.strong_count() == 1 {
            memo.remove(&self.hash);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MemoizedValue {
    bool_value: Option<bool>,
    json_value: Option<HashMap<String, JsonValue>>,
}

impl MemoizedValue {
    fn new(raw_json: &str) -> Arc<Self> {
        let value = match raw_json {
            "true" | "false" => Self::from_bool(raw_json == "true"),
            raw_json => Self::from_object_str(raw_json),
        };

        Arc::new(value)
    }

    fn from_bool(bool_value: bool) -> Self {
        Self {
            bool_value: Some(bool_value),
            json_value: None,
        }
    }

    fn from_object_str(raw_json: &str) -> Self {
        let json_value = match serde_json::from_str(raw_json) {
            Ok(json_value) => json_value,
            Err(e) => {
                log_e!(TAG, "Failed to parse json: {}", e);
                None
            }
        };

        Self {
            bool_value: None,
            json_value,
        }
    }
}

fn get_memoized_value(hash: &str) -> Option<Arc<MemoizedValue>> {
    let mut memoized_values = match MEMOIZED_VALUES.lock() {
        Ok(values) => values,
        Err(e) => {
            log_e!(TAG, "Failed to lock memoized values: {}", e);
            return None;
        }
    };

    let found = memoized_values.get(hash)?;

    match found.upgrade() {
        Some(value) => Some(value),
        None => {
            memoized_values.remove(hash);
            None
        }
    }
}

fn set_memoized_value(hash: &str, value: Weak<MemoizedValue>) {
    match MEMOIZED_VALUES.lock() {
        Ok(mut values) => {
            values.insert(hash.to_string(), value);
        }
        Err(e) => {
            log_e!(TAG, "Failed to lock memoized values: {}", e);
        }
    };
}
