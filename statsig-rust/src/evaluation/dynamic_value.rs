use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};
use std::{collections::HashMap, fmt::Debug};

use crate::{
    hashing::{self, ahash_str},
    log_w,
};

use super::dynamic_string::DynamicString;

const TAG: &str = "DynamicValue";

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

    #[must_use]
    pub(crate) fn for_int_evaluation(value: i64) -> DynamicValue {
        DynamicValue {
            int_value: Some(value),
            float_value: Some(value as f64),
            string_value: Some(DynamicString::from(value.to_string())),
            json_value: JsonValue::Number(serde_json::Number::from(value)),
            ..DynamicValue::default()
        }
    }

    #[must_use]
    pub(crate) fn for_numeric_comparison_evaluation(value: i64) -> DynamicValue {
        DynamicValue {
            int_value: Some(value),
            float_value: Some(value as f64),
            ..DynamicValue::default()
        }
    }

    #[must_use]
    pub(crate) fn for_string_membership_evaluation(value: i64) -> DynamicValue {
        DynamicValue {
            string_value: Some(DynamicString::from(value.to_string())),
            ..DynamicValue::default()
        }
    }

    fn try_parse_timestamp(s: &str) -> Option<i64> {
        // Fast-path: try parsing as integer first
        if let Ok(ts) = s.parse::<i64>() {
            return Some(ts);
        }

        // Fast-reject: if the string is out of range or lacks typical date delimiters
        if s.len() < 8 || s.len() > 20 || (!s.contains('-') && !s.contains('T') && !s.contains(':'))
        {
            return None;
        }

        // Try RFC3339 (e.g. "2023-01-01T12:00:00Z")
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis());
        }

        // Try common datetime format (e.g. "2023-01-01 12:00:00")
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
        // perf optimization: avoid stringifying the json value if it's already a string
        let mut stringified_json_value = None;
        let hash_value = if let JsonValue::String(s) = &json_value {
            ahash_str(s)
        } else {
            let actual = json_value.to_string();
            let hash = ahash_str(&actual);
            stringified_json_value = Some(actual);
            hash
        };

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
                let mut float_value = n.as_f64();
                let mut int_value = n.as_i64();
                if let (Some(f), None) = (float_value, int_value) {
                    let iv = f as i64;
                    if iv as f64 == f {
                        int_value = Some(iv);
                    }
                } else if let (None, Some(i)) = (float_value, int_value) {
                    let fv = i as f64;
                    if fv as i64 == i {
                        float_value = Some(fv)
                    }
                }

                let string_value = float_value
                    .map(|f| f.to_string())
                    .or_else(|| int_value.map(|i| i.to_string()))
                    .or(stringified_json_value);

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
                string_value: Some(DynamicString::from(
                    stringified_json_value.unwrap_or(json_value.to_string()),
                )),
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
        Self::from(serde_json::Value::String(value))
    }
}

impl From<&str> for DynamicValue {
    fn from(value: &str) -> Self {
        Self::from(serde_json::Value::String(value.to_string()))
    }
}

impl From<usize> for DynamicValue {
    fn from(value: usize) -> Self {
        Self::from(serde_json::Value::Number(serde_json::Number::from(value)))
    }
}

impl From<i64> for DynamicValue {
    fn from(value: i64) -> Self {
        Self::from(serde_json::Value::Number(serde_json::Number::from(value)))
    }
}

impl From<i32> for DynamicValue {
    fn from(value: i32) -> Self {
        Self::from(serde_json::Value::Number(serde_json::Number::from(value)))
    }
}

impl From<f64> for DynamicValue {
    fn from(value: f64) -> Self {
        let num = match serde_json::Number::from_f64(value) {
            Some(num) => num,
            None => {
                log_w!(
                    TAG,
                    "Failed to convert f64 to serde_json::Number: {}",
                    value
                );
                serde_json::Number::from(value as i64)
            }
        };

        Self::from(serde_json::Value::Number(num))
    }
}

impl From<bool> for DynamicValue {
    fn from(value: bool) -> Self {
        Self::from(serde_json::Value::Bool(value))
    }
}

impl From<Vec<JsonValue>> for DynamicValue {
    fn from(value: Vec<JsonValue>) -> Self {
        DynamicValue::from(serde_json::Value::Array(value))
    }
}

impl From<Vec<DynamicValue>> for DynamicValue {
    fn from(value: Vec<DynamicValue>) -> Self {
        let json_value = json!(value);
        let string_value = DynamicString::from(json_value.to_string());
        let hash_value = hashing::hash_one(value.iter().map(|v| v.hash_value).collect::<Vec<_>>());

        DynamicValue {
            hash_value,
            array_value: Some(value),
            json_value,
            string_value: Some(string_value),
            ..DynamicValue::default()
        }
    }
}

impl From<HashMap<String, DynamicValue>> for DynamicValue {
    fn from(value: HashMap<String, DynamicValue>) -> Self {
        let json_value = json!(value);
        let hash_value =
            hashing::hash_one(value.values().map(|v| v.hash_value).collect::<Vec<_>>());

        DynamicValue {
            hash_value,
            object_value: Some(value),
            json_value,
            ..DynamicValue::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicValue;
    use crate::{
        evaluation::comparisons::{compare_numbers, compare_strings_in_array},
        test_only_make_eval_value,
    };

    #[test]
    fn for_int_evaluation_matches_generic_number_fields() {
        let specialized = DynamicValue::for_int_evaluation(123);
        let generic = DynamicValue::from(123);

        assert_eq!(specialized.int_value, generic.int_value);
        assert_eq!(specialized.float_value, generic.float_value);
        assert_eq!(specialized.string_value, generic.string_value);
        assert_eq!(specialized.json_value, generic.json_value);
    }

    #[test]
    fn for_numeric_comparison_evaluation_matches_generic_number_comparisons() {
        let specialized = DynamicValue::for_numeric_comparison_evaluation(123);
        let generic = DynamicValue::from(123);
        let target = test_only_make_eval_value!(500);

        assert_eq!(
            compare_numbers(&specialized, &target, "lt"),
            compare_numbers(&generic, &target, "lt")
        );
        assert_eq!(
            compare_numbers(&specialized, &target, "gte"),
            compare_numbers(&generic, &target, "gte")
        );
    }

    #[test]
    fn for_string_membership_evaluation_matches_generic_string_membership() {
        let specialized = DynamicValue::for_string_membership_evaluation(123);
        let generic = DynamicValue::from(123);
        let target = test_only_make_eval_value!(vec!["122", "123", "124"]);

        assert_eq!(
            compare_strings_in_array(&specialized, &target, "any", true),
            compare_strings_in_array(&generic, &target, "any", true)
        );
        assert_eq!(
            compare_strings_in_array(&specialized, &target, "none", true),
            compare_strings_in_array(&generic, &target, "none", true)
        );
    }
}
