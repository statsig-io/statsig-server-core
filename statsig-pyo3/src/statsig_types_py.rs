use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;
use serde_json::{json, Map, Value};
use statsig_rust::{
    statsig_types::{DynamicConfig, Experiment, Layer},
    DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
    LayerEvaluationOptions,
};

#[gen_stub_pyclass]
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

#[gen_stub_pyclass]
#[pyclass(name = "DynamicConfig")]
pub struct DynamicConfigPy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub value: PyObject,

    pub inner: DynamicConfig,
}

#[gen_stub_pyclass]
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
    pub value: PyObject,

    pub inner: Experiment,
}

#[gen_stub_pyclass]
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
    pub value: PyObject,

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

#[gen_stub_pyclass]
#[pyclass(name = "FeatureGateEvaluationOptions")]
#[derive(FromPyObject)]
pub struct FeatureGateEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

#[gen_stub_pyclass]
#[pyclass(name = "DynamicConfigEvaluationOptions")]
#[derive(FromPyObject)]
pub struct DynamicConfigEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

#[gen_stub_pyclass]
#[pyclass(name = "ExperimentEvaluationOptions")]
#[derive(FromPyObject)]
pub struct ExperimentEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

#[gen_stub_pyclass]
#[pyclass(name = "LayerEvaluationOptions")]
#[derive(FromPyObject)]
pub struct LayerEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

impl From<FeatureGateEvaluationOptionsPy> for FeatureGateEvaluationOptions {
    fn from(val: FeatureGateEvaluationOptionsPy) -> FeatureGateEvaluationOptions {
        FeatureGateEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
        }
    }
}

impl From<DynamicConfigEvaluationOptionsPy> for DynamicConfigEvaluationOptions {
    fn from(val: DynamicConfigEvaluationOptionsPy) -> DynamicConfigEvaluationOptions {
        DynamicConfigEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
        }
    }
}

impl From<ExperimentEvaluationOptionsPy> for ExperimentEvaluationOptions {
    fn from(val: ExperimentEvaluationOptionsPy) -> ExperimentEvaluationOptions {
        ExperimentEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
        }
    }
}

impl From<LayerEvaluationOptionsPy> for LayerEvaluationOptions {
    fn from(val: LayerEvaluationOptionsPy) -> LayerEvaluationOptions {
        LayerEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
        }
    }
}

macro_rules! impl_new_method {
    ($struct_name:ident) => {
        #[gen_stub_pymethods]
        #[pymethods]
        impl $struct_name {
            #[new]
            #[pyo3(signature = (disable_exposure_logging=false))]
            pub fn new(disable_exposure_logging: bool) -> Self {
                Self {
                    disable_exposure_logging,
                }
            }
        }
    };
}

impl_new_method!(FeatureGateEvaluationOptionsPy);
impl_new_method!(DynamicConfigEvaluationOptionsPy);
impl_new_method!(ExperimentEvaluationOptionsPy);
impl_new_method!(LayerEvaluationOptionsPy);
