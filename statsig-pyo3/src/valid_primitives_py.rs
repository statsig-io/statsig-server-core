use pyo3::prelude::*;
use pyo3_stub_gen::{PyStubType, TypeInfo};
use serde_json::Value;
use statsig_rust::{dyn_value, log_w, DynamicValue};
use std::{
    collections::{HashMap, HashSet},
    str,
};

const TAG: &str = stringify!(ValidPrimitivesPy);

#[derive(FromPyObject, IntoPyObject, Clone)]
pub enum ValidArrayItemPy {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl ValidArrayItemPy {
    pub fn into_dynamic_value(self) -> DynamicValue {
        match self {
            ValidArrayItemPy::String(s) => dyn_value!(s),
            ValidArrayItemPy::Int(i) => dyn_value!(i),
            ValidArrayItemPy::Float(f) => dyn_value!(f),
            ValidArrayItemPy::Bool(b) => dyn_value!(b),
        }
    }
}

#[derive(IntoPyObject)]
pub enum ValidArrayItemPyRef<'a> {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(&'a str),
}

#[derive(IntoPyObject, Clone)]
pub enum ValidPrimitivesPy {
    Bool(bool),
    Float(f64),
    Int(i64),
    Array(Vec<ValidArrayItemPy>),
    String(String),
    Dictionary(HashMap<String, Option<ValidPrimitivesPy>>),
}

impl<'py> FromPyObject<'py> for ValidPrimitivesPy {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(b) = ob.extract::<bool>() {
            return Ok(ValidPrimitivesPy::Bool(b));
        }

        if let Ok(i) = ob.extract::<i64>() {
            return Ok(ValidPrimitivesPy::Int(i));
        }

        if let Ok(f) = ob.extract::<f64>() {
            return Ok(ValidPrimitivesPy::Float(f));
        }

        if let Ok(s) = ob.extract::<Vec<ValidArrayItemPy>>() {
            return Ok(ValidPrimitivesPy::Array(s));
        }

        if let Ok(s) = ob.extract::<String>() {
            return Ok(ValidPrimitivesPy::String(s));
        }

        if let Ok(dict) = ob.extract::<HashMap<String, Option<ValidPrimitivesPy>>>() {
            return Ok(ValidPrimitivesPy::Dictionary(dict));
        }

        log_w!(
            TAG,
            "Invalid ValidPrimitivesPy: {:?}. Valid types are: int, float, bool, str, list[str, int, float, bool], dict[str, Union[str, int, float, bool]]. Defaulting to empty string.",
            ob
        );

        Ok(ValidPrimitivesPy::String(String::new()))
    }
}

impl PyStubType for ValidPrimitivesPy {
    fn type_output() -> TypeInfo {
        TypeInfo {
            name: "typing.Union[builtins.str, builtins.int, builtins.float, builtins.bool, typing.List[typing.Union[builtins.str, builtins.int, builtins.float, builtins.bool]]]".to_string(),
            import: HashSet::from(["builtins".into(), "typing".into()]),
        }
    }
}

impl ValidPrimitivesPy {
    pub fn into_dynamic_value(self) -> DynamicValue {
        match self {
            ValidPrimitivesPy::Int(i) => dyn_value!(i),
            ValidPrimitivesPy::Float(f) => dyn_value!(f),
            ValidPrimitivesPy::Bool(b) => dyn_value!(b),
            ValidPrimitivesPy::String(s) => dyn_value!(s),
            ValidPrimitivesPy::Array(a) => {
                let mapped = a
                    .into_iter()
                    .map(ValidArrayItemPy::into_dynamic_value)
                    .collect::<Vec<_>>();
                dyn_value!(mapped)
            }
            ValidPrimitivesPy::Dictionary(dict) => {
                let converted: HashMap<String, DynamicValue> = dict
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                Some(v) => v.into_dynamic_value(),
                                None => DynamicValue::new(),
                            },
                        )
                    })
                    .collect();
                dyn_value!(converted)
            }
        }
    }
}

#[derive(IntoPyObject)]
pub enum ValidPrimitivesPyRef<'a> {
    Bool(bool),
    Float(f64),
    Int(i64),
    Array(Vec<ValidArrayItemPyRef<'a>>),
    String(&'a str),
    Dictionary(HashMap<String, Option<ValidPrimitivesPyRef<'a>>>),
}

impl PyStubType for ValidPrimitivesPyRef<'_> {
    fn type_output() -> TypeInfo {
        TypeInfo {
            name: "typing.Union[builtins.str, builtins.int, builtins.float, builtins.bool, typing.List[typing.Union[builtins.str, builtins.int, builtins.float, builtins.bool]]]".to_string(),
            import: HashSet::from(["builtins".into(), "typing".into()]),
        }
    }
}

impl<'a> ValidPrimitivesPyRef<'a> {
    pub fn from_dynamic_value(value: &'a DynamicValue) -> Option<Self> {
        let json_value = &value.json_value;

        ValidPrimitivesPyRef::from_value(json_value)
    }

    pub fn from_value(json_value: &'a Value) -> Option<Self> {
        if let Value::String(v) = json_value {
            return Some(ValidPrimitivesPyRef::String(v.as_str()));
        }

        if let Value::Number(v) = json_value {
            if let Some(v) = v.as_i64() {
                return Some(ValidPrimitivesPyRef::Int(v));
            } else if let Some(v) = v.as_f64() {
                return Some(ValidPrimitivesPyRef::Float(v));
            }
        }

        if let Value::Bool(v) = json_value {
            return Some(ValidPrimitivesPyRef::Bool(*v));
        }

        if let Value::Array(v) = json_value {
            let mapped = v
                .iter()
                .map(|v| json_value_to_valid_array_item_py(v))
                .collect::<Vec<_>>();

            return Some(ValidPrimitivesPyRef::Array(mapped));
        }

        if let Value::Object(v) = json_value {
            let mapped = v
                .iter()
                .map(|(k, v)| (k.clone(), ValidPrimitivesPyRef::from_value(v)))
                .collect::<HashMap<_, _>>();

            return Some(ValidPrimitivesPyRef::Dictionary(mapped));
        }

        None
    }
}

fn json_value_to_valid_array_item_py(value: &Value) -> ValidArrayItemPyRef<'_> {
    match value {
        Value::String(v) => ValidArrayItemPyRef::String(v.as_str()),
        Value::Number(v) => {
            if let Some(v) = v.as_i64() {
                ValidArrayItemPyRef::Int(v)
            } else if let Some(v) = v.as_f64() {
                ValidArrayItemPyRef::Float(v)
            } else {
                ValidArrayItemPyRef::String("")
            }
        }
        Value::Bool(v) => ValidArrayItemPyRef::Bool(*v),
        _ => ValidArrayItemPyRef::String(""),
    }
}
