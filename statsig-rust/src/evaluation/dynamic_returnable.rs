use crate::{hashing::djb2, log_e};
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value as JsonValue,
};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};

const TAG: &str = "DynamicReturnable";

lazy_static::lazy_static! {
    pub(crate) static ref MEMOIZED_VALUES: Mutex<HashMap<String, Weak<MemoizedValue>>> =
        Mutex::new(HashMap::new());

    static ref EMPTY_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: "".to_string(),
        value: Arc::new(MemoizedValue {
            raw_value: None,
            bool_value: None,
            json_value: None,
        }),
    };
}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicReturnable {
    hash: String,
    value: Arc<MemoizedValue>,
}

impl DynamicReturnable {
    pub fn empty() -> Self {
        EMPTY_DYNAMIC_RETURNABLE.clone()
    }

    pub fn from_map(value: HashMap<String, JsonValue>) -> Self {
        let raw_value = match to_raw_value(&value) {
            Ok(raw_value) => raw_value,
            Err(e) => {
                log_e!(TAG, "Failed to convert map to raw value: {}", e);
                return Self::empty();
            }
        };

        let hash = djb2(raw_value.get());
        let value = Arc::new(MemoizedValue {
            raw_value: Some(raw_value),
            bool_value: None,
            json_value: Some(value.clone()),
        });

        Self::new(hash.to_string(), value)
    }

    pub fn get_bool(&self) -> Option<bool> {
        self.value.bool_value
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        self.value.json_value.clone()
    }

    pub fn get_json_ref(&self) -> Option<&HashMap<String, JsonValue>> {
        self.value.json_value.as_ref()
    }

    fn new(hash: String, value: Arc<MemoizedValue>) -> Self {
        let weak_value = Arc::downgrade(&value);
        set_memoized_value(&hash, weak_value);

        Self { hash, value }
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value_ref: &'de RawValue = Deserialize::deserialize(deserializer)?;

        let raw_value_str = raw_value_ref.get();
        let hash = djb2(raw_value_str);

        if let Some(value) = get_memoized_value(&hash) {
            return Ok(DynamicReturnable { hash, value });
        }

        let raw_value = raw_value_ref.to_owned();
        let value = MemoizedValue::new(raw_value);

        let new_returnable = DynamicReturnable::new(hash.clone(), value);

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

        if let Some(raw_value) = &self.value.raw_value {
            return raw_value.serialize(serializer);
        }

        if let Some(json_value) = &self.value.json_value {
            return json_value.serialize(serializer);
        }

        JsonValue::Null.serialize(serializer)
    }
}

impl Drop for DynamicReturnable {
    fn drop(&mut self) {
        let mut memo = match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(1)) {
            Some(values) => values,
            None => {
                log_e!(
                    TAG,
                    "Failed to lock memoized values: Failed to lock MEMOIZED_VALUES"
                );
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

#[derive(Debug, Clone)]
pub(crate) struct MemoizedValue {
    pub(crate) raw_value: Option<Box<RawValue>>,
    pub(crate) bool_value: Option<bool>,
    pub(crate) json_value: Option<HashMap<String, JsonValue>>,
}

impl PartialEq for MemoizedValue {
    fn eq(&self, other: &Self) -> bool {
        self.raw_value.as_ref().map(|v| v.get()) == other.raw_value.as_ref().map(|v| v.get())
            && self.bool_value == other.bool_value
            && self.json_value == other.json_value
    }
}

impl MemoizedValue {
    fn new(raw_value: Box<RawValue>) -> Arc<Self> {
        let value = match raw_value.get() {
            "true" => Self::from_bool(true),
            "false" => Self::from_bool(false),
            _ => Self::from_raw_value(raw_value),
        };

        Arc::new(value)
    }

    fn from_bool(bool_value: bool) -> Self {
        Self {
            raw_value: None,
            bool_value: Some(bool_value),
            json_value: None,
        }
    }

    fn from_raw_value(raw_value: Box<RawValue>) -> Self {
        let json_value = match serde_json::from_str(raw_value.get()) {
            Ok(json_value) => json_value,
            Err(e) => {
                log_e!(TAG, "Failed to parse json: {}", e);
                None
            }
        };

        Self {
            raw_value: Some(raw_value),
            bool_value: None,
            json_value,
        }
    }
}

fn get_memoized_value(hash: &str) -> Option<Arc<MemoizedValue>> {
    let mut memoized_values = match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(1)) {
        Some(values) => values,
        None => {
            log_e!(
                TAG,
                "Failed to lock memoized values: Failed to lock MEMOIZED_VALUES"
            );
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
    match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(1)) {
        Some(mut values) => {
            values.insert(hash.to_string(), value);
        }
        None => {
            log_e!(
                TAG,
                "Failed to lock memoized values: Failed to lock MEMOIZED_VALUES"
            );
        }
    };
}
