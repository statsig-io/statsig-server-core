use std::{borrow::Cow, collections::HashMap, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value as JsonValue,
};

use crate::{impl_interned_value, interned_value_store::FromRawValue, log_e};

const TAG: &str = "DynamicReturnable";

lazy_static::lazy_static! {
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

impl_interned_value!(DynamicReturnable, MemoizedValue);

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

        let (hash, value) = DynamicReturnable::get_or_create_memoized(Cow::Owned(raw_value));
        Self { hash, value }
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self.value.raw_value.get() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        let raw_json = self.value.raw_value.get();

        match raw_json {
            "null" | "true" | "false" => None,
            _ => match serde_json::from_str(raw_json) {
                Ok(json) => Some(json),
                Err(e) => {
                    log_e!(TAG, "Failed to parse json: {}. Error: {}", raw_json, e);
                    None
                }
            },
        }
    }

    pub fn get_hash(&self) -> u64 {
        self.hash
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value_ref: Box<RawValue> = Deserialize::deserialize(deserializer)?;
        let (hash, value) = DynamicReturnable::get_or_create_memoized(Cow::Owned(raw_value_ref));
        Ok(DynamicReturnable { hash, value })
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

#[derive(Debug, Clone)]
pub(crate) struct MemoizedValue {
    pub(crate) raw_value: Box<RawValue>,
}

impl FromRawValue for MemoizedValue {
    fn from_raw_value(raw_value: Cow<'_, RawValue>) -> Self {
        Self {
            raw_value: raw_value.into_owned(),
        }
    }
}

impl PartialEq for MemoizedValue {
    fn eq(&self, other: &Self) -> bool {
        self.raw_value.get() == other.raw_value.get()
    }
}
