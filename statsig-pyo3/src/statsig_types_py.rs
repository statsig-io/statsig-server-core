use pyo3::prelude::*;
use serde_json::{json, Map, Value};
use sigstat::statsig_types::{DynamicConfig, Experiment, Layer};

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

#[pyclass(name = "DynamicConfig")]
pub struct DynamicConfigPy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub value: String,

    pub inner: DynamicConfig,
}

#[pyclass(name = "Experiment")]
pub struct ExperimentPy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub group_name: Option<String>,

    #[pyo3(get)]
    pub value: String,

    pub inner: Experiment,
}

#[pyclass(name = "Layer")]
pub struct LayerPy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub group_name: Option<String>,

    #[pyo3(get)]
    pub allocated_experiment_name: Option<String>,

    #[pyo3(get)]
    pub value: String,

    pub inner: Layer,
}

macro_rules! impl_get_methods {
    ($struct_name:ident) => {
        #[pymethods]
        impl $struct_name {
            pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
                self.inner.get(param_name, fallback)
            }

            pub fn get_float(&self, param_name: &str, fallback: f64) -> f64 {
                self.inner.get(param_name, fallback)
            }

            pub fn get_integer(&self, param_name: &str, fallback: i64) -> i64 {
                self.inner.get(param_name, fallback)
            }

            pub fn get_string(&self, param_name: &str, fallback: String) -> String {
                self.inner.get(param_name, fallback)
            }

            pub fn get_array_json(&self, param_name: &str, fallback: String) -> String {
                let result = match self.inner.get_opt::<Vec<Value>>(param_name) {
                    Some(v) => v,
                    None => return fallback,
                };

                json!(result).to_string()
            }

            pub fn get_object_json(&self, param_name: &str, fallback: String) -> String {
                let result = match self.inner.get_opt::<Map<String, Value>>(param_name) {
                    Some(v) => v,
                    None => return fallback,
                };

                json!(result).to_string()
            }
        }
    };
}

impl_get_methods!(DynamicConfigPy);
impl_get_methods!(ExperimentPy);
impl_get_methods!(LayerPy);
