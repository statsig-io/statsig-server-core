use std::collections::HashMap;

use pyo3::prelude::*;
use sigstat::DynamicValue;

#[derive(IntoPyObject)]
pub enum JsonPrimitive {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<JsonPrimitive>),
    Object(HashMap<String, JsonPrimitive>),
}

#[pyclass(name = "FeatureGate")]
pub struct FeatureGatePy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub value: bool,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,
}

#[pyclass(name = "Experiment")]
pub struct ExperimentPy {
    #[pyo3(get)]
    pub name: String,

    pub value: HashMap<String, DynamicValue>,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub group_name: Option<String>,
}

#[pymethods]
impl ExperimentPy {
    #[pyo3(signature = (key, fallback=None))]
    pub fn get_string(&self, key: &str, fallback: Option<&str>) -> Option<JsonPrimitive> {
        self.value
            .get(key)
            .and_then(|v| {
                v.string_value
                    .as_ref()
                    .map(|s| JsonPrimitive::String(s.to_string()))
            })
            .or_else(|| fallback.map(|f| JsonPrimitive::String(f.to_string())))
    }

    #[pyo3(signature = (key, fallback=None))]
    pub fn get_number(&self, key: &str, fallback: Option<f64>) -> Option<JsonPrimitive> {
        self.value
            .get(key)
            .and_then(|v| v.float_value.as_ref().map(|f| JsonPrimitive::Number(*f)))
            .or_else(|| fallback.map(|f| JsonPrimitive::Number(f)))
    }

    #[pyo3(signature = (key, fallback=None))]
    pub fn get_bool(&self, key: &str, fallback: Option<bool>) -> Option<JsonPrimitive> {
        self.value
            .get(key)
            .and_then(|v| v.bool_value.as_ref().map(|b| JsonPrimitive::Bool(*b)))
            .or_else(|| fallback.map(|f| JsonPrimitive::Bool(f)))
    }
}
