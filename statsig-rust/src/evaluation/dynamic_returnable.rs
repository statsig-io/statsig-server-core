use crate::{hashing, log_e};
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
    pub(crate) static ref MEMOIZED_VALUES: Mutex<HashMap<u64, Weak<MemoizedValue>>> =
        Mutex::new(HashMap::new());

    static ref EMPTY_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: Arc::new(MemoizedValue {
            raw_value: RawValue::NULL.to_owned(),
        }),
    };
}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicReturnable {
    hash: u64,
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

        let hash = hashing::hash_one(raw_value.get());

        if let Some(value) = get_memoized_value(hash) {
            return DynamicReturnable {
                hash,
                value: value.clone(),
            };
        }

        let value = MemoizedValue::new(raw_value);
        Self::new(hash, value)
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self.value.raw_value.get() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        match serde_json::from_str(self.value.raw_value.get()) {
            Ok(json) => Some(json),
            Err(e) => {
                log_e!(
                    TAG,
                    "Failed to parse json: {}. Error: {}",
                    self.value.raw_value.get(),
                    e
                );
                None
            }
        }
    }

    fn new(hash: u64, value: Arc<MemoizedValue>) -> Self {
        let weak_value = Arc::downgrade(&value);
        set_memoized_value(hash, weak_value);

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
        let hash = hashing::hash_one(raw_value_str);

        if let Some(value) = get_memoized_value(hash) {
            return Ok(DynamicReturnable { hash, value });
        }

        let raw_value = raw_value_ref.to_owned();
        let value = MemoizedValue::new(raw_value);

        let new_returnable = DynamicReturnable::new(hash, value);
        Ok(new_returnable)
    }
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.raw_value.serialize(serializer)
    }
}

impl Drop for DynamicReturnable {
    fn drop(&mut self) {
        let mut memo = match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(5)) {
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
    pub(crate) raw_value: Box<RawValue>,
}

impl PartialEq for MemoizedValue {
    fn eq(&self, other: &Self) -> bool {
        self.raw_value.get() == other.raw_value.get()
    }
}

impl MemoizedValue {
    fn new(raw_value: Box<RawValue>) -> Arc<Self> {
        Arc::new(Self { raw_value })
    }
}

fn get_memoized_value(hash: u64) -> Option<Arc<MemoizedValue>> {
    let mut memoized_values = match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(5)) {
        Some(values) => values,
        None => {
            log_e!(
                TAG,
                "Failed to lock memoized values: Failed to lock MEMOIZED_VALUES"
            );
            return None;
        }
    };

    let found = memoized_values.get(&hash)?;

    match found.upgrade() {
        Some(value) => Some(value),
        None => {
            memoized_values.remove(&hash);
            None
        }
    }
}

fn set_memoized_value(hash: u64, value: Weak<MemoizedValue>) {
    match MEMOIZED_VALUES.try_lock_for(Duration::from_secs(5)) {
        Some(mut values) => {
            values.insert(hash, value);
        }
        None => {
            log_e!(
                TAG,
                "Failed to lock memoized values: Failed to lock MEMOIZED_VALUES"
            );
        }
    };
}
