use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};
use std::{collections::HashMap, fmt::Debug};

use crate::hashing::ahash_str;

use super::dynamic_string::DynamicString;

#[macro_export]
macro_rules! dyn_value {
    ($x:expr) => {{
        $crate::DynamicValue::from_json_value($x)
    }};
}

#[derive(Debug, Clone, Default)]
pub struct DynamicValue {
    pub null: Option<()>,
    pub bool_value: Option<bool>,
    pub int_value: Option<i64>,
    pub float_value: Option<f64>,
    pub timestamp_value: Option<i64>,
    pub string_value: Option<DynamicString>,
    pub array_value: Option<Vec<DynamicValue>>,
    pub object_value: Option<HashMap<String, DynamicValue>>,
    pub json_value: JsonValue,
    pub hash_value: u64,
}

impl DynamicValue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_json_value(value: impl Serialize) -> Self {
        Self::from(json!(value))
    }

    #[must_use]
    pub fn for_timestamp_evaluation(timestamp: i64) -> DynamicValue {
        DynamicValue {
            int_value: Some(timestamp),
            ..DynamicValue::default()
        }
    }

    fn try_parse_timestamp(s: &str) -> Option<i64> {
        if let Ok(ts) = s.parse::<i64>() {
            return Some(ts);
        }

        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis());
        }

        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Some(ndt.and_utc().timestamp_millis());
        }

        None
    }
}

impl PartialEq for DynamicValue {
    fn eq(&self, other: &Self) -> bool {
        self.null == other.null
            && self.bool_value == other.bool_value
            && self.int_value == other.int_value
            && self.float_value == other.float_value
            && self.string_value == other.string_value
            && self.array_value == other.array_value
            && self.object_value == other.object_value
    }
}

// ------------------------------------------------------------------------------- [Serialization]

impl Serialize for DynamicValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.json_value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DynamicValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = JsonValue::deserialize(deserializer)?;
        Ok(DynamicValue::from(json_value))
    }
}

// ------------------------------------------------------------------------------- [From<T> Implementations]

impl From<JsonValue> for DynamicValue {
    fn from(json_value: JsonValue) -> Self {
        let hash_value = ahash_str(&json_value.to_string());

        match &json_value {
            JsonValue::Null => DynamicValue {
                null: Some(()),
                json_value,
                hash_value,
                ..DynamicValue::new()
            },

            JsonValue::Bool(b) => DynamicValue {
                bool_value: Some(*b),
                string_value: Some(DynamicString::from(b.to_string())),
                json_value,
                hash_value,
                ..DynamicValue::new()
            },

            JsonValue::Number(n) => {
                let float_value = n.as_f64();
                let int_value = n.as_i64();

                let string_value = float_value
                    .map(|f| f.to_string())
                    .or_else(|| int_value.map(|i| i.to_string()))
                    .or_else(|| Some(json_value.to_string()));

                DynamicValue {
                    float_value,
                    int_value,
                    string_value: string_value.map(DynamicString::from),
                    json_value,
                    hash_value,
                    ..DynamicValue::new()
                }
            }

            JsonValue::String(s) => {
                let timestamp_value = Self::try_parse_timestamp(s);
                let float_value = s.parse().ok();
                let int_value = s.parse().ok();
                DynamicValue {
                    string_value: Some(DynamicString::from(s.clone())),
                    json_value,
                    timestamp_value,
                    int_value,
                    float_value,
                    hash_value,
                    ..DynamicValue::new()
                }
            }

            JsonValue::Array(arr) => DynamicValue {
                array_value: Some(arr.iter().map(|v| DynamicValue::from(v.clone())).collect()),
                string_value: Some(DynamicString::from(json_value.to_string())),
                json_value,
                hash_value,
                ..DynamicValue::new()
            },

            JsonValue::Object(obj) => DynamicValue {
                object_value: Some(
                    obj.into_iter()
                        .map(|(k, v)| (k.clone(), DynamicValue::from(v.clone())))
                        .collect(),
                ),
                json_value,
                hash_value,
                ..DynamicValue::new()
            },
        }
    }
}

impl From<String> for DynamicValue {
    fn from(value: String) -> Self {
        Self::from(json!(value))
    }
}

impl From<&str> for DynamicValue {
    fn from(value: &str) -> Self {
        Self::from(json!(value))
    }
}

impl From<usize> for DynamicValue {
    fn from(value: usize) -> Self {
        Self::from(json!(value))
    }
}

impl From<i64> for DynamicValue {
    fn from(value: i64) -> Self {
        Self::from(json!(value))
    }
}

impl From<i32> for DynamicValue {
    fn from(value: i32) -> Self {
        Self::from(json!(value))
    }
}

impl From<f64> for DynamicValue {
    fn from(value: f64) -> Self {
        Self::from(json!(value))
    }
}

impl From<bool> for DynamicValue {
    fn from(value: bool) -> Self {
        Self::from(json!(value))
    }
}

impl From<Vec<JsonValue>> for DynamicValue {
    fn from(value: Vec<JsonValue>) -> Self {
        DynamicValue::from(json!(value))
    }
}
