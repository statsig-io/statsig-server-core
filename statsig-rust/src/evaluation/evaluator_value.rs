use chrono::{DateTime, NaiveDateTime};
use regex_lite::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Value as JsonValue, Value};
use std::collections::HashMap;

use crate::{unwrap_or_return, DynamicValue};

use super::dynamic_string::DynamicString;

#[macro_export]
macro_rules! test_only_make_eval_value {
    ($x:expr) => {
        $crate::evaluation::evaluator_value::EvaluatorValue::from(serde_json::json!($x))
    };
}

#[derive(Debug, PartialEq)]
pub enum EvaluatorValueType {
    Null,

    Bool,
    Number,
    String,
    Array,
    Object,
}

#[derive(Debug)]
pub struct EvaluatorValue {
    pub value_type: EvaluatorValueType,
    pub bool_value: Option<bool>,
    pub float_value: Option<f64>,
    pub string_value: Option<DynamicString>,
    pub regex_value: Option<Regex>,
    pub timestamp_value: Option<i64>,
    // { lower_case_str: (index, str) } -- Keyed by lowercase string so we can lookup with O(1)
    pub array_value: Option<HashMap<String, (usize, String)>>,
    pub object_value: Option<HashMap<String, DynamicString>>,
}

impl EvaluatorValue {
    pub fn new(value_type: EvaluatorValueType) -> Self {
        Self {
            value_type,
            bool_value: None,
            float_value: None,
            string_value: None,
            regex_value: None,
            timestamp_value: None,
            array_value: None,
            object_value: None,
        }
    }

    pub fn null() -> Self {
        Self::new(EvaluatorValueType::Null)
    }

    pub fn compile_regex(&mut self) {
        let str_value = match &self.string_value {
            Some(dyn_str) => &dyn_str.value,
            None => return,
        };

        if let Ok(regex) = Regex::new(str_value) {
            self.regex_value = Some(regex);
        }
    }

    pub fn is_equal_to_dynamic_value(&self, other: &DynamicValue) -> bool {
        match self.value_type {
            EvaluatorValueType::Null => other.json_value == Value::Null,
            EvaluatorValueType::Bool => self.bool_value == other.bool_value,
            EvaluatorValueType::Number => self.float_value == other.float_value,
            EvaluatorValueType::String => self.string_value == other.string_value,
            EvaluatorValueType::Array => {
                let self_keyed_arr = match &self.array_value {
                    Some(map) => map,
                    None => return other.array_value.is_none(),
                };

                let other_arr = match &other.array_value {
                    Some(arr) => arr,
                    None => return false,
                };

                if self_keyed_arr.len() != other_arr.len() {
                    return false;
                }

                for (i, self_value) in self_keyed_arr.values() {
                    let other_dyn_str = unwrap_or_return!(&other_arr[*i].string_value, false);
                    if *self_value != other_dyn_str.value {
                        return false;
                    }
                }

                true
            }
            EvaluatorValueType::Object => {
                let self_obj = match &self.object_value {
                    Some(map) => map,
                    None => return other.object_value.is_none(),
                };

                let other_obj = match &other.object_value {
                    Some(arr) => arr,
                    None => return false,
                };

                if self_obj.len() != other_obj.len() {
                    return false;
                }

                for (k, v) in self_obj {
                    let other_dyn_val = unwrap_or_return!(other_obj.get(k), false);
                    let other_str_val = unwrap_or_return!(&other_dyn_val.string_value, false);
                    if other_str_val.value != v.value {
                        return false;
                    }
                }

                true
            }
        }
    }
}

// Used during evaluation:
// - ua_parser
impl From<String> for EvaluatorValue {
    fn from(value: String) -> Self {
        EvaluatorValue {
            timestamp_value: try_parse_timestamp(&value),
            float_value: value.parse::<f64>().ok(),
            string_value: Some(DynamicString::from(value)),
            ..EvaluatorValue::new(EvaluatorValueType::String)
        }
    }
}

// Used during Deserialization
impl From<JsonValue> for EvaluatorValue {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => EvaluatorValue::new(EvaluatorValueType::Null),

            JsonValue::Bool(b) => EvaluatorValue {
                bool_value: Some(b),
                ..EvaluatorValue::new(EvaluatorValueType::Bool)
            },

            JsonValue::Number(n) => EvaluatorValue {
                float_value: n.as_f64(),
                ..EvaluatorValue::new(EvaluatorValueType::Number)
            },

            JsonValue::String(s) => EvaluatorValue::from(s),

            JsonValue::Array(arr) => {
                let keyed_array: HashMap<String, (usize, String)> = arr
                    .into_iter()
                    .enumerate()
                    .map(|(idx, val)| {
                        let str_value = match val.as_str() {
                            Some(s) => s.to_string(), // Value is a String
                            None => val.to_string(),  // Value was not a String, but can be made one
                        };

                        (str_value.to_lowercase(), (idx, str_value))
                    })
                    .collect();

                EvaluatorValue {
                    array_value: Some(keyed_array),
                    ..EvaluatorValue::new(EvaluatorValueType::Array)
                }
            }

            JsonValue::Object(obj) => EvaluatorValue {
                object_value: Some(
                    obj.into_iter()
                        .map(|(k, v)| (k, DynamicString::from(v)))
                        .collect(),
                ),
                ..EvaluatorValue::new(EvaluatorValueType::Object)
            },
        }
    }
}

impl Serialize for EvaluatorValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value_type {
            EvaluatorValueType::Null => JsonValue::Null.serialize(serializer),
            EvaluatorValueType::Bool => self.bool_value.serialize(serializer),
            EvaluatorValueType::Number => self.float_value.serialize(serializer),
            EvaluatorValueType::String => self.string_value.serialize(serializer),
            EvaluatorValueType::Array => self
                .array_value
                .as_ref()
                .map(|x| x.values().collect::<Vec<_>>())
                .serialize(serializer),
            EvaluatorValueType::Object => self.object_value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for EvaluatorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = JsonValue::deserialize(deserializer)?;
        Ok(EvaluatorValue::from(json_value))
    }
}

impl PartialEq for EvaluatorValue {
    fn eq(&self, other: &Self) -> bool {
        self.value_type == other.value_type
            && self.bool_value == other.bool_value
            && self.float_value == other.float_value
            && self.string_value == other.string_value
            && self.array_value == other.array_value
            && self.object_value == other.object_value
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
