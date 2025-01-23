use crate::statsig_options_py::StatsigOptionsPy;
use crate::statsig_types_py::{DynamicConfigPy, LayerPy};
use crate::{
    statsig_types_py::{ExperimentPy, FeatureGatePy, FeatureGateEvaluationOptionsPy, ExperimentEvaluationOptionsPy, DynamicConfigEvaluationOptionsPy, LayerEvaluationOptionsPy},
    statsig_user_py::StatsigUserPy,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sigstat::{log_e, unwrap_or_return, ClientInitResponseOptions, ExperimentEvaluationOptions, LayerEvaluationOptions, DynamicConfigEvaluationOptions, FeatureGateEvaluationOptions, HashAlgorithm, Statsig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const TAG: &str = stringify!(StatsigPy);

#[pyclass(eq, eq_int, name = "StatsigResult")]
#[derive(PartialEq)]
pub enum StatsigResultPy {
    Ok,
    InvalidKey,
    NoDice,
}

#[pyclass(name = "Statsig")]
pub struct StatsigPy {
    inner: Arc<Statsig>,
}

#[pymethods]
impl StatsigPy {
    #[new]
    #[pyo3(signature = (sdk_key, options=None))]
    pub fn new(sdk_key: &str, options: Option<&StatsigOptionsPy>) -> Self {
        let mut local_opts = None;
        if let Some(o) = options {
            local_opts = Some(Arc::new(o.into()));
        }

        Self {
            inner: Arc::new(Statsig::new(sdk_key, local_opts)),
        }
    }

    pub fn initialize(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        self.inner.statsig_runtime.runtime_handle.spawn(async move {
            if let Err(e) = inst.initialize().await {
                log_e!(TAG, "Failed to initialize Statsig: {}", e);
            }

            Python::with_gil(|py| {
                if let Err(e) = event_clone.call_method0(py, "set") {
                    log_e!(TAG, "Failed to set event: {}", e);
                }
            });
        });

        Ok(completion_event)
    }

    pub fn flush_events(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        self.inner.statsig_runtime.runtime_handle.spawn(async move {
            inst.flush_events().await;

            Python::with_gil(|py| {
                if let Err(e) = event_clone.call_method0(py, "set") {
                    log_e!(TAG, "Failed to set event: {}", e);
                }
            });
        });

        Ok(completion_event)
    }

    pub fn shutdown(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        let rt = self.inner.statsig_runtime.clone();
        rt.runtime_handle.spawn(async move {
            if let Err(e) = inst.__shutdown_internal(Duration::from_secs(3)).await {
                log_e!(TAG, "Failed to gracefully shutdown StatsigPy: {}", e);
            }

            Python::with_gil(|py| {
                event_clone.call_method0(py, "set").unwrap();
            });
        });

        Ok(completion_event)
    }

    #[pyo3(signature = (user, event_name, value=None, metadata=None))]
    pub fn log_event(
        &self,
        user: &StatsigUserPy,
        event_name: &str,
        value: Option<Bound<PyAny>>,
        metadata: Option<Bound<PyDict>>,
    ) -> PyResult<()> {
        let local_metadata = extract_event_metadata(metadata);

        if let Some(num_value) = convert_to_number(value.as_ref()) {
            self.inner.log_event_with_number(
                &user.inner,
                event_name,
                Some(num_value),
                local_metadata,
            );
        } else {
            self.inner.log_event(
                &user.inner,
                event_name,
                convert_to_string(value.as_ref()),
                local_metadata,
            );
        }

        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn check_gate(&self, user: &StatsigUserPy, name: &str, options: Option<&FeatureGateEvaluationOptionsPy>) -> bool {
        self.inner.check_gate_with_options(&user.inner, name, options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()))
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_feature_gate(&self, user: &StatsigUserPy, name: &str, options: Option<&FeatureGateEvaluationOptionsPy>) -> FeatureGatePy {
        let gate = self.inner.get_feature_gate_with_options(&user.inner, name, options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()));
        FeatureGatePy {
            name: gate.name,
            value: gate.value,
            rule_id: gate.rule_id,
            id_type: gate.id_type,
        }
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_gate_exposure(&self, user: &StatsigUserPy, name: &str) -> PyResult<()> {
        self.inner.manually_log_gate_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_dynamic_config(&self, user: &StatsigUserPy, name: &str, options: Option<&DynamicConfigEvaluationOptionsPy>) -> DynamicConfigPy {
        let config = self.inner.get_dynamic_config_with_options(&user.inner, name, options.map_or(DynamicConfigEvaluationOptions::default(), |o| o.into()));

        DynamicConfigPy {
            name: config.name.clone(),
            rule_id: config.rule_id.clone(),
            id_type: config.id_type.clone(),
            value: serde_json::to_string(&config.value).unwrap(),
            inner: config,
        }
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_dynamic_config_exposure(&self, user: &StatsigUserPy, name: &str) -> PyResult<()> {
        self.inner.manually_log_dynamic_config_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_experiment(&self, user: &StatsigUserPy, name: &str, options: Option<&ExperimentEvaluationOptionsPy>) -> ExperimentPy {
        let experiment = self.inner.get_experiment_with_options(&user.inner, name, options.map_or(ExperimentEvaluationOptions::default(), |o| o.into()));

        ExperimentPy {
            name: experiment.name.clone(),
            rule_id: experiment.rule_id.clone(),
            id_type: experiment.id_type.clone(),
            group_name: experiment.group_name.clone(),
            value: serde_json::to_string(&experiment.value).unwrap(),
            inner: experiment,
        }
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_experiment_exposure(&self, user: &StatsigUserPy, name: &str) -> PyResult<()> {
        self.inner.manually_log_experiment_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_layer(&self, user: &StatsigUserPy, name: &str, options: Option<&LayerEvaluationOptionsPy>) -> LayerPy {
        let layer = self.inner.get_layer_with_options(&user.inner, name, options.map_or(LayerEvaluationOptions::default(), |o| o.into()));

        LayerPy {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            allocated_experiment_name: layer.allocated_experiment_name.clone(),
            value: serde_json::to_string(&layer.__value).unwrap(),
            inner: layer,
        }
    }

    #[pyo3(signature = (user, name, param_name))]
    pub fn manually_log_layer_parameter_exposure(&self, user: &StatsigUserPy, name: &str, param_name: String) -> PyResult<()> {
        self.inner.manually_log_layer_parameter_exposure(&user.inner, name, param_name);
        Ok(())
    }

    #[pyo3(signature = (user, hash=None, client_sdk_key=None))]
    pub fn get_client_init_response(&self, user: &StatsigUserPy, hash: Option<&str>, client_sdk_key: Option<&str>) -> String {
        let mut opts = ClientInitResponseOptions::default();
        if hash == Some("none") {
            opts.hash_algorithm = Some(HashAlgorithm::None);
        }
        if hash == Some("sha256") {
            opts.hash_algorithm = Some(HashAlgorithm::Sha256);
        }
        if let Some(client_sdk_key) = client_sdk_key {
            opts.client_sdk_key = Some(client_sdk_key.to_string());
        }
        self.inner
            .get_client_init_response_with_options_as_string(&user.inner, &opts)
    }
}

fn get_completion_event(py: Python) -> PyResult<(PyObject, PyObject)> {
    let threading = PyModule::import(py, "threading")?;
    let event = threading.call_method0("Event")?;
    let event_clone: PyObject = event.clone().into();

    Ok((event.into(), event_clone))
}

fn convert_to_number(value: Option<&Bound<PyAny>>) -> Option<f64> {
    let value = value?;

    value.extract::<f64>().ok()
}

fn convert_to_string(value: Option<&Bound<PyAny>>) -> Option<String> {
    let value = value?;

    value.extract::<String>().ok()
}

fn extract_event_metadata(metadata: Option<Bound<PyDict>>) -> Option<HashMap<String, String>> {
    if let Some(m) = metadata {
        let mut local_map = HashMap::new();

        for (k, v) in m.iter() {
            let key: String = unwrap_or_return!(k.extract().ok(), None);
            let value: String = unwrap_or_return!(v.extract().ok(), None);
            local_map.insert(key, value);
        }

        return Some(local_map);
    }

    None
}
