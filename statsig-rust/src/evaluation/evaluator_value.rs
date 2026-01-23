use chrono::{DateTime, NaiveDateTime};
use fancy_regex::Regex as FancyRegex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value as JsonValue, Value,
};
use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::{
    interned_string::InternedString, interned_values::InternedStore, log_e, unwrap_or_return,
    DynamicValue,
};

use super::dynamic_string::DynamicString;

lazy_static::lazy_static! {
    pub(crate) static ref EMPTY_EVALUATOR_VALUE: EvaluatorValue = EvaluatorValue {
        hash: 0,
        inner: EvaluatorValueInner::Pointer(Arc::new(MemoizedEvaluatorValue::new(EvaluatorValueType::Null))),
    };
}

const TAG: &str = "EvaluatorValue";

#[derive(Clone, Debug)]
pub enum EvaluatorValueInner {
    Pointer(Arc<MemoizedEvaluatorValue>),
    Static(&'static MemoizedEvaluatorValue),
}

#[derive(Clone, Debug)]
pub struct EvaluatorValue {
    pub hash: u64,
    pub inner: EvaluatorValueInner,
}

impl EvaluatorValue {
    pub fn empty() -> &'static Self {
        &EMPTY_EVALUATOR_VALUE
    }

    pub fn from_json_value(value: Value) -> Self {
        let raw_value = match to_raw_value(&value) {
            Ok(raw_value) => raw_value,
            Err(e) => {
                log_e!(TAG, "Failed to convert map to raw value: {}", e);
                return Self::empty().clone();
            }
        };

        InternedStore::get_or_intern_evaluator_value(Cow::Owned(raw_value))
    }

    pub fn compile_regex(&mut self) {
        match &mut self.inner {
            EvaluatorValueInner::Pointer(inner) => {
                let mut_inner = Arc::make_mut(inner);
                mut_inner.compile_regex();
            }
            EvaluatorValueInner::Static(_) => {
                // static values are immutable and are compiled during `InternedStore::bootstrap(..)`
                log_e!(TAG, "Cannot compile regex for static EvaluatorValue");
            }
        }
    }
}

impl AsRef<MemoizedEvaluatorValue> for EvaluatorValue {
    fn as_ref(&self) -> &MemoizedEvaluatorValue {
        match &self.inner {
            EvaluatorValueInner::Pointer(inner) => inner,
            EvaluatorValueInner::Static(inner) => inner,
        }
    }
}

impl<'de> Deserialize<'de> for EvaluatorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value_ref: Box<RawValue> = Deserialize::deserialize(deserializer)?;
        Ok(InternedStore::get_or_intern_evaluator_value(Cow::Owned(
            raw_value_ref,
        )))
    }
}

impl Serialize for EvaluatorValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.inner {
            EvaluatorValueInner::Pointer(inner) => inner.serialize(serializer),
            EvaluatorValueInner::Static(inner) => inner.serialize(serializer),
        }
    }
}

impl PartialEq for EvaluatorValue {
    fn eq(&self, other: &Self) -> bool {
        let left = match &self.inner {
            EvaluatorValueInner::Pointer(inner) => inner,
            EvaluatorValueInner::Static(inner) => *inner,
        };
        let right = match &other.inner {
            EvaluatorValueInner::Pointer(inner) => inner,
            EvaluatorValueInner::Static(inner) => *inner,
        };

        left == right
    }
}

impl Drop for EvaluatorValue {
    fn drop(&mut self) {
        self.inner = EMPTY_EVALUATOR_VALUE.inner.clone();
        InternedStore::release_evaluator_value(self.hash);
    }
}

// ------------------------------------------------------------------------------- [ MemoizedEvaluatorValue ]

#[derive(Debug, PartialEq, Clone)]
pub enum EvaluatorValueType {
    Null,

    Bool,
    Number,
    String,
    Array,
    Object,
}

#[derive(Debug, Clone)]
pub struct MemoizedEvaluatorValue {
    pub value_type: EvaluatorValueType,
    pub bool_value: Option<bool>,
    pub float_value: Option<f64>,
    pub string_value: Option<DynamicString>,
    pub regex_value: Option<FancyRegex>,
    pub timestamp_value: Option<i64>,
    pub object_value: Option<HashMap<InternedString, DynamicString>>,

    // - Note on Array Value ------------------------------------------------------------
    // - Keyed by lowercase string so we can lookup with O(1) during evaluation.
    // - Format is `{ lower_case_str: (index, str) }` i.e: ["Apple", "Banana"] becomes { "apple": (0, "Apple"), "banana": (1, "Banana") }
    // - The index is what position in the array it is, currently this is only used to serialzie back to the original JSON.
    // ----------------------------------------------------------------------------------
    pub array_value: Option<HashMap<InternedString, (usize, InternedString)>>,
}

impl MemoizedEvaluatorValue {
    pub fn from_raw_value(raw_value: Cow<'_, RawValue>) -> Self {
        match serde_json::from_str(raw_value.get()) {
            Ok(value) => value,
            Err(e) => {
                log_e!(
                    TAG,
                    "Failed to convert raw value to MemoizedEvaluatorValue: {}",
                    e
                );
                Self::null()
            }
        }
    }
}

impl MemoizedEvaluatorValue {
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

        if let Ok(regex) = FancyRegex::new(str_value) {
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
                    let other_dyn_val = unwrap_or_return!(other_obj.get(k.as_str()), false);
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
impl From<String> for MemoizedEvaluatorValue {
    fn from(value: String) -> Self {
        MemoizedEvaluatorValue {
            timestamp_value: try_parse_timestamp(&value),
            float_value: value.parse::<f64>().ok(),
            string_value: Some(DynamicString::from(value)),
            ..MemoizedEvaluatorValue::new(EvaluatorValueType::String)
        }
    }
}

// Used during Deserialization
impl From<JsonValue> for MemoizedEvaluatorValue {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => MemoizedEvaluatorValue::new(EvaluatorValueType::Null),

            JsonValue::Bool(b) => MemoizedEvaluatorValue {
                bool_value: Some(b),
                ..MemoizedEvaluatorValue::new(EvaluatorValueType::Bool)
            },

            JsonValue::Number(n) => MemoizedEvaluatorValue {
                float_value: n.as_f64(),
                ..MemoizedEvaluatorValue::new(EvaluatorValueType::Number)
            },

            JsonValue::String(s) => MemoizedEvaluatorValue::from(s),

            JsonValue::Array(arr) => {
                let keyed_array: HashMap<InternedString, (usize, InternedString)> = arr
                    .into_iter()
                    .enumerate()
                    .map(|(idx, val)| {
                        let str_value = match val.as_str() {
                            Some(s) => s.to_string(), // Value is a String
                            None => val.to_string(),  // Value was not a String, but can be made one
                        };

                        let interned_lowercased_str =
                            InternedString::from_string(str_value.to_lowercase());
                        let interned_str = InternedString::from_string(str_value);

                        (interned_lowercased_str, (idx, interned_str))
                    })
                    .collect();

                MemoizedEvaluatorValue {
                    array_value: Some(keyed_array),
                    ..MemoizedEvaluatorValue::new(EvaluatorValueType::Array)
                }
            }

            JsonValue::Object(obj) => MemoizedEvaluatorValue {
                object_value: Some(
                    obj.into_iter()
                        .map(|(k, v)| (InternedString::from_string(k), DynamicString::from(v)))
                        .collect(),
                ),
                ..MemoizedEvaluatorValue::new(EvaluatorValueType::Object)
            },
        }
    }
}

impl Serialize for MemoizedEvaluatorValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value_type {
            EvaluatorValueType::Null => JsonValue::Null.serialize(serializer),
            EvaluatorValueType::Bool => self.bool_value.serialize(serializer),
            EvaluatorValueType::Number => self.float_value.serialize(serializer),
            EvaluatorValueType::String => self.string_value.serialize(serializer),
            EvaluatorValueType::Array => {
                let array_map = match &self.array_value {
                    Some(a) => a,
                    None => return JsonValue::Null.serialize(serializer),
                };

                let mut result = vec![String::new(); array_map.len()];

                for (idx, val) in array_map.values() {
                    result[*idx] = val.unperformant_to_string();
                }

                result.serialize(serializer)
            }
            EvaluatorValueType::Object => self.object_value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MemoizedEvaluatorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = JsonValue::deserialize(deserializer)?;
        Ok(MemoizedEvaluatorValue::from(json_value))
    }
}

impl PartialEq for MemoizedEvaluatorValue {
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

#[macro_export]
macro_rules! test_only_make_eval_value {
    ($x:expr) => {
        $crate::evaluation::evaluator_value::MemoizedEvaluatorValue::from(serde_json::json!($x))
    };
}
