use std::{borrow::Cow, collections::HashMap, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value as JsonValue,
};

use crate::{interned_values::InternedStore, log_e};

const TAG: &str = "DynamicReturnable";

lazy_static::lazy_static! {
    static ref EMPTY_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Null,
    };

    static ref TRUE_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Bool(true),
    };

    static ref FALSE_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Bool(false),
    };

}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicReturnable {
    pub hash: u64,
    pub value: DynamicReturnableValue,
}

impl DynamicReturnable {
    pub fn empty() -> Self {
        EMPTY_DYNAMIC_RETURNABLE.clone()
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            TRUE_DYNAMIC_RETURNABLE.clone()
        } else {
            FALSE_DYNAMIC_RETURNABLE.clone()
        }
    }

    pub fn from_map(value: HashMap<String, JsonValue>) -> Self {
        let raw_value = match to_raw_value(&value) {
            Ok(raw_value) => raw_value,
            Err(e) => {
                log_e!(TAG, "Failed to convert map to raw value: {}", e);
                return Self::empty();
            }
        };

        InternedStore::get_or_intern_returnable(Cow::Owned(raw_value))
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self.value {
            DynamicReturnableValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        let bytes = match &self.value {
            DynamicReturnableValue::JsonPointer(bytes) => bytes.get().as_bytes(),
            DynamicReturnableValue::JsonStatic(bytes) => bytes.get().as_bytes(),
            _ => return None,
        };

        match serde_json::from_slice(bytes) {
            Ok(json) => Some(json),
            Err(e) => {
                log_e!(TAG, "Failed to parse json. Error: {}", e);
                None
            }
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
        let raw_value_ref: Cow<RawValue> = Deserialize::deserialize(deserializer)?;
        Ok(InternedStore::get_or_intern_returnable(raw_value_ref))
    }
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value {
            DynamicReturnableValue::JsonPointer(raw) => raw.serialize(serializer),
            DynamicReturnableValue::JsonStatic(raw) => raw.serialize(serializer),
            DynamicReturnableValue::Null => serializer.serialize_none(),
            DynamicReturnableValue::Bool(value) => serializer.serialize_bool(*value),
        }
    }
}

impl Drop for DynamicReturnable {
    fn drop(&mut self) {
        self.value = DynamicReturnableValue::Null;
        InternedStore::release_returnable(self.hash);
    }
}

// ------------------------------------------------------------------------------- [ DynamicReturnableValue ]

#[derive(Clone, Debug)]
pub enum DynamicReturnableValue {
    Null,
    Bool(bool),
    JsonPointer(Arc<Box<RawValue>>),
    JsonStatic(&'static RawValue),
}

impl DynamicReturnableValue {
    fn raw_string_value(&self) -> Option<&str> {
        match self {
            DynamicReturnableValue::JsonPointer(raw) => Some(raw.as_ref().get()),
            DynamicReturnableValue::JsonStatic(raw) => Some(raw.get()),
            _ => None,
        }
    }
}

impl PartialEq for DynamicReturnableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DynamicReturnableValue::Null, DynamicReturnableValue::Null) => true,
            (DynamicReturnableValue::Bool(a), DynamicReturnableValue::Bool(b)) => *a == *b,
            (left, right) => left.raw_string_value() == right.raw_string_value(),
        }
    }
}
