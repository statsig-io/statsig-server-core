use std::sync::Weak;

use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};
use pyo3_stub_gen::derive::*;
use serde_json::{json, Map, Value};
use statsig_rust::{
    log_e,
    statsig_types::{DynamicConfig, Experiment, Layer},
    DynamicConfigEvaluationOptions, EvaluationDetails, ExperimentEvaluationOptions, FailureDetails,
    FeatureGateEvaluationOptions, InitializeDetails, LayerEvaluationOptions,
    ParameterStoreEvaluationOptions, Statsig, StatsigUser,
};

use crate::pyo_utils::{
    list_of_values_to_py_list, map_to_py_dict, py_dict_to_json_value_map, py_list_to_list_of_values,
};

const TAG: &str = stringify!(StatsigTypesPy);

#[gen_stub_pyclass]
#[pyclass(name = "InitializeDetails", module = "statsig_python_core")]
pub struct InitializeDetailsPy {
    #[pyo3(get)]
    pub duration: f64, // deprecated, this should be removed in the next major version. Use duration_ms instead.

    #[pyo3(get)]
    pub duration_ms: u64,

    #[pyo3(get)]
    pub init_success: bool,

    #[pyo3(get)]
    pub is_config_spec_ready: bool,

    #[pyo3(get)]
    pub is_id_list_ready: Option<bool>,

    #[pyo3(get)]
    pub source: String,

    #[pyo3(get)]
    pub failure_details: Option<FailureDetailsPy>,
}

impl From<InitializeDetails> for InitializeDetailsPy {
    fn from(value: InitializeDetails) -> Self {
        InitializeDetailsPy {
            duration_ms: value.duration_ms,
            init_success: value.init_success,
            is_config_spec_ready: value.is_config_spec_ready,
            is_id_list_ready: value.is_id_list_ready,
            source: value.source.to_string(),
            failure_details: value.failure_details.map(FailureDetailsPy::from),

            #[allow(deprecated)]
            duration: value.duration_ms as f64,
        }
    }
}

impl InitializeDetailsPy {
    pub fn from_error(reason: &str, error: Option<String>) -> Self {
        InitializeDetailsPy {
            duration_ms: 0,
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: "error".to_string(),
            failure_details: Some(FailureDetailsPy {
                reason: reason.to_string(),
                error,
            }),

            #[allow(deprecated)]
            duration: 0.0,
        }
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "FailureDetails", module = "statsig_python_core")]
#[derive(Clone)]
pub struct FailureDetailsPy {
    #[pyo3(get)]
    pub reason: String,

    #[pyo3(get)]
    pub error: Option<String>,
}

impl From<FailureDetails> for FailureDetailsPy {
    fn from(value: FailureDetails) -> Self {
        FailureDetailsPy {
            reason: value.reason,
            error: value.error.map(|e| e.to_string()),
        }
    }
}
#[gen_stub_pyclass]
#[pyclass(name = "EvaluationDetails", module = "statsig_python_core")]
#[derive(Clone)]
pub struct EvaluationDetailsPy {
    #[pyo3(get)]
    pub reason: String,

    #[pyo3(get)]
    pub lcut: Option<u64>,

    #[pyo3(get)]
    pub received_at: Option<u64>,

    #[pyo3(get)]
    pub version: Option<u32>,
}

impl From<EvaluationDetails> for EvaluationDetailsPy {
    fn from(value: EvaluationDetails) -> Self {
        EvaluationDetailsPy {
            reason: value.reason,
            lcut: value.lcut,
            received_at: value.received_at,
            version: value.version,
        }
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "FeatureGate", module = "statsig_python_core")]
pub struct FeatureGatePy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub value: bool,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub details: EvaluationDetailsPy,
}

#[gen_stub_pyclass]
#[pyclass(name = "DynamicConfig", module = "statsig_python_core")]
pub struct DynamicConfigPy {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub rule_id: String,

    #[pyo3(get)]
    pub id_type: String,

    #[pyo3(get)]
    pub value: PyObject,

    #[pyo3(get)]
    pub details: EvaluationDetailsPy,

    pub inner: DynamicConfig,
}

#[gen_stub_pyclass]
#[pyclass(name = "Experiment", module = "statsig_python_core")]
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

    #[pyo3(get)]
    pub details: EvaluationDetailsPy,

    pub inner: Experiment,
}

#[gen_stub_pyclass]
#[pyclass(name = "Layer", module = "statsig_python_core")]
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

    #[pyo3(get)]
    pub details: EvaluationDetailsPy,

    pub inner: Layer,
}

#[gen_stub_pyclass]
#[pyclass(name = "ParameterStore", module = "statsig_python_core")]
pub struct ParameterStorePy {
    #[pyo3(get)]
    pub name: String,

    pub inner_statsig: Weak<Statsig>,
    pub user: StatsigUser,
    pub options: ParameterStoreEvaluationOptions,
}

#[gen_stub_pymethods]
#[pymethods]
impl ParameterStorePy {
    pub fn get_string(&self, param_name: &str, fallback: String) -> String {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => inner_statsig
                .get_string_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(fallback.clone()),
                    Some(self.options),
                )
                .unwrap_or(fallback),
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback
            }
        }
    }

    pub fn get_float(&self, param_name: &str, fallback: f64) -> f64 {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => inner_statsig
                .get_float_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(fallback),
                    Some(self.options),
                )
                .unwrap_or(fallback),
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback
            }
        }
    }

    pub fn get_integer(&self, param_name: &str, fallback: i64) -> i64 {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => inner_statsig
                .get_integer_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(fallback),
                    Some(self.options),
                )
                .unwrap_or(fallback),
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback
            }
        }
    }

    pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => inner_statsig
                .get_boolean_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(fallback),
                    Some(self.options),
                )
                .unwrap_or(fallback),
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback
            }
        }
    }

    pub fn get_array(&self, py: Python, param_name: &str, fallback: Bound<PyList>) -> PyObject {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => {
                let result = inner_statsig.get_array_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(py_list_to_list_of_values(&fallback).unwrap_or_default()),
                    Some(self.options),
                );
                if let Some(result) = result {
                    match list_of_values_to_py_list(py, &result) {
                        Ok(list) => list,
                        Err(_) => fallback.into(),
                    }
                } else {
                    fallback.into()
                }
            }
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback.into()
            }
        }
    }

    pub fn get_map(&self, py: Python, param_name: &str, fallback: Bound<PyDict>) -> PyObject {
        match self.inner_statsig.upgrade() {
            Some(inner_statsig) => {
                let result = inner_statsig.get_object_parameter_from_store(
                    &self.user,
                    &self.name,
                    param_name,
                    Some(py_dict_to_json_value_map(&fallback)),
                    Some(self.options),
                );
                if let Some(result) = result {
                    map_to_py_dict(py, &result)
                } else {
                    fallback.into()
                }
            }
            None => {
                log_e!(TAG, "Failed to upgrade Statsig instance");
                fallback.into()
            }
        }
    }
}

macro_rules! impl_get_methods {
    ($struct_name:ident) => {
        #[gen_stub_pymethods]
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
#[pyclass(name = "FeatureGateEvaluationOptions", module = "statsig_python_core")]
#[derive(FromPyObject)]
pub struct FeatureGateEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

#[gen_stub_pyclass]
#[pyclass(
    name = "DynamicConfigEvaluationOptions",
    module = "statsig_python_core"
)]
#[derive(FromPyObject)]
pub struct DynamicConfigEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
}

#[gen_stub_pyclass]
#[pyclass(name = "ExperimentEvaluationOptions", module = "statsig_python_core")]
#[derive(FromPyObject)]
pub struct ExperimentEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
    #[pyo3(get, set)]
    pub user_persisted_values: Option<Py<PyDict>>,
}

#[gen_stub_pyclass]
#[pyclass(name = "LayerEvaluationOptions", module = "statsig_python_core")]
#[derive(FromPyObject)]
pub struct LayerEvaluationOptionsPy {
    #[pyo3(get)]
    pub disable_exposure_logging: bool,
    #[pyo3(get, set)]
    pub user_persisted_values: Option<Py<PyDict>>,
}

#[gen_stub_pyclass]
#[pyclass(
    name = "ParameterStoreEvaluationOptions",
    module = "statsig_python_core"
)]
#[derive(FromPyObject)]
pub struct ParameterStoreEvaluationOptionsPy {
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

impl From<&ExperimentEvaluationOptionsPy> for ExperimentEvaluationOptions {
    fn from(val: &ExperimentEvaluationOptionsPy) -> ExperimentEvaluationOptions {
        ExperimentEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
            // For performance consideration, conversion to user persisted values use convert_dict_to_user_persisted_values()
            user_persisted_values: None,
        }
    }
}

impl From<&LayerEvaluationOptionsPy> for LayerEvaluationOptions {
    fn from(val: &LayerEvaluationOptionsPy) -> LayerEvaluationOptions {
        LayerEvaluationOptions {
            disable_exposure_logging: val.disable_exposure_logging,
            // For performance consideration, conversion to user persisted values use convert_dict_to_user_persisted_values()
            user_persisted_values: None,
        }
    }
}

impl From<ParameterStoreEvaluationOptionsPy> for ParameterStoreEvaluationOptions {
    fn from(val: ParameterStoreEvaluationOptionsPy) -> ParameterStoreEvaluationOptions {
        ParameterStoreEvaluationOptions {
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

macro_rules! impl_new_method_with_persisted_values {
    ($struct_name:ident) => {
        #[gen_stub_pymethods]
        #[pymethods]
        impl $struct_name {
            #[new]
            #[pyo3(signature = (disable_exposure_logging=false, user_persisted_values=None))]
            pub fn new(
                disable_exposure_logging: bool,
                user_persisted_values: Option<Py<PyDict>>,
            ) -> Self {
                Self {
                    disable_exposure_logging,
                    user_persisted_values,
                }
            }
        }
    };
}

impl_new_method!(FeatureGateEvaluationOptionsPy);
impl_new_method!(DynamicConfigEvaluationOptionsPy);
impl_new_method!(ParameterStoreEvaluationOptionsPy);
impl_new_method_with_persisted_values!(ExperimentEvaluationOptionsPy);
impl_new_method_with_persisted_values!(LayerEvaluationOptionsPy);
