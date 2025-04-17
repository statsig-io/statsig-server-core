use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

#[derive(Clone, PartialEq, Debug)]
pub enum DynamicReturnable {
    Bool(bool),
    Json(HashMap<String, JsonValue>),
}

impl DynamicReturnable {
    pub fn get_bool(&self) -> Option<bool> {
        match self {
            DynamicReturnable::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        match self {
            DynamicReturnable::Json(json) => Some(json.clone()),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = JsonValue::deserialize(deserializer)?;
        match json_value {
            JsonValue::Bool(b) => Ok(DynamicReturnable::Bool(b)),
            JsonValue::Object(obj) => Ok(DynamicReturnable::Json(obj.into_iter().collect())),
            _ => Err(serde::de::Error::custom("Invalid JSON value")),
        }
    }
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DynamicReturnable::Bool(b) => b.serialize(serializer),
            DynamicReturnable::Json(json) => json.serialize(serializer),
        }
    }
}
