use crate::pyo_utils::py_dict_to_json_value_map;
use crate::safe_gil::SafeGil;
use crate::statsig_options_py::{safe_convert_to_statsig_options, StatsigOptionsPy};
use crate::statsig_persistent_storage_override_adapter_py::convert_dict_to_user_persisted_values;
use crate::statsig_types_py::{
    InitializeDetailsPy, ParameterStoreEvaluationOptionsPy, ParameterStorePy,
};
use crate::{
    statsig_types_py::{
        DynamicConfigEvaluationOptionsPy, ExperimentEvaluationOptionsPy,
        FeatureGateEvaluationOptionsPy, LayerEvaluationOptionsPy,
    },
    statsig_user_py::StatsigUserPy,
};
use parking_lot::Mutex;
use pyo3::types::PyTuple;
use pyo3::{prelude::*, types::PyDict};
use pyo3_stub_gen::derive::*;
use serde_json::Value;
use statsig_rust::{
    log_e, ClientInitResponseOptions, DynamicConfigEvaluationOptions, ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions, HashAlgorithm, LayerEvaluationOptions, ObservabilityClient,
    ParameterStoreEvaluationOptions, Statsig, UserPersistedValues,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const TAG: &str = stringify!(StatsigBasePy);

#[gen_stub_pyclass]
#[pyclass(module = "statsig_python_core", subclass)]
pub struct StatsigBasePy {
    inner: Arc<Statsig>,
    observability_client: Mutex<Option<Arc<dyn ObservabilityClient>>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StatsigBasePy {
    #[new]
    #[pyo3(signature = (sdk_key, options=None))]
    pub fn new(sdk_key: &str, options: Option<StatsigOptionsPy>, py: Python) -> Self {
        let (opts, ob_client) = safe_convert_to_statsig_options(py, options);

        Self {
            inner: Arc::new(Statsig::new(sdk_key, opts.map(Arc::new))),
            observability_client: Mutex::new(ob_client),
        }
    }

    pub fn initialize(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        let spawn_result = self.inner.statsig_runtime.spawn(TAG, |_| async move {
            if let Err(e) = inst.initialize().await {
                log_e!(TAG, "Failed to initialize Statsig: {}", e);
            }

            SafeGil::run(|py| {
                let py = match py {
                    Some(py) => py,
                    None => return,
                };

                call_completion_event(&event_clone, py);
            });
        });

        if let Err(e) = spawn_result {
            log_e!(TAG, "Failed to spawn statsig initialize task: {e}");
            call_completion_event(&completion_event, py);
        }

        Ok(completion_event)
    }

    pub fn initialize_with_details(&self, py: Python) -> PyResult<PyObject> {
        let (future, future_clone) = create_python_future(py)?;

        let inst = self.inner.clone();
        let spawn_result = self.inner.statsig_runtime.spawn(TAG, |_| async move {
            let result = inst.initialize_with_details().await;

            SafeGil::run(|py| {
                let py = match py {
                    Some(py) => py,
                    None => return,
                };

                match result {
                    Ok(details) => {
                        let py_details = InitializeDetailsPy::from(details);
                        call_completion_future(&future_clone, py, (py_details,));
                    }
                    Err(e) => {
                        let error_details = InitializeDetailsPy::from_error(
                            "initialize_failed",
                            Some(e.to_string()),
                        );
                        call_completion_future(&future_clone, py, (error_details,));
                    }
                };
            });
        });

        if let Err(e) = spawn_result {
            log_e!(
                TAG,
                "Failed to spawn statsig initialize with details task: {e}"
            );
            let error_details =
                InitializeDetailsPy::from_error("initialize_failed", Some(e.to_string()));
            call_completion_future(&future, py, (error_details,));
        }

        Ok(future)
    }

    pub fn get_initialize_details(&self) -> PyResult<InitializeDetailsPy> {
        let details = self.inner.get_initialize_details();
        let py_details = InitializeDetailsPy::from(details);
        Ok(py_details)
    }

    pub fn is_initialized(&self) -> bool {
        self.inner.is_initialized()
    }

    pub fn flush_events(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        let spawn_result = self.inner.statsig_runtime.spawn(TAG, |_| async move {
            inst.flush_events().await;

            SafeGil::run(|py| {
                let py = match py {
                    Some(py) => py,
                    None => return,
                };

                call_completion_event(&event_clone, py);
            });
        });

        if let Err(e) = spawn_result {
            log_e!(TAG, "Failed to spawn statsig flush events task: {e}");
            call_completion_event(&completion_event, py);
        }

        Ok(completion_event)
    }

    pub fn shutdown(&self, py: Python) -> PyResult<PyObject> {
        let (completion_event, event_clone) = get_completion_event(py)?;

        let inst = self.inner.clone();
        let obs_client = match self
            .observability_client
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => lock.take(),
            None => {
                log_e!(TAG, "Failed to lock observability client");
                None
            }
        };

        let spawn_result = self.inner.statsig_runtime.spawn(TAG, |_| async move {
            if let Err(e) = inst.shutdown().await {
                log_e!(TAG, "Failed to gracefully shutdown StatsigPy: {}", e);
            }

            SafeGil::run(|py| {
                let py = match py {
                    Some(py) => py,
                    None => return,
                };

                call_completion_event(&event_clone, py);
            });

            // held until the shutdown is complete
            drop(obs_client);
        });

        if let Err(e) = spawn_result {
            log_e!(TAG, "Failed to spawn statsig shutdown task: {e}");
            call_completion_event(&completion_event, py);
        }

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
            self.inner.log_event_with_number_and_typed_metadata(
                &user.inner,
                event_name,
                Some(num_value),
                local_metadata,
            );
        } else {
            self.inner.log_event_with_typed_metadata(
                &user.inner,
                event_name,
                convert_to_string(value.as_ref()),
                local_metadata,
            );
        }

        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn check_gate(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<FeatureGateEvaluationOptionsPy>,
    ) -> bool {
        self.inner.check_gate_with_options(
            &user.inner,
            name,
            options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()),
        )
    }

    #[pyo3(name="_INTERNAL_get_feature_gate", signature = (user, name, options=None))]
    pub fn _internal_get_feature_gate(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<FeatureGateEvaluationOptionsPy>,
    ) -> String {
        self.inner.get_raw_feature_gate_with_options(
            &user.inner,
            name,
            options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()),
        )
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_gate_exposure(&self, user: &StatsigUserPy, name: &str) -> PyResult<()> {
        self.inner.manually_log_gate_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(name="_INTERNAL_get_dynamic_config", signature = (user, name, options=None))]
    pub fn _internal_get_dynamic_config(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<DynamicConfigEvaluationOptionsPy>,
    ) -> String {
        self.inner.get_raw_dynamic_config_with_options(
            &user.inner,
            name,
            options.map_or(DynamicConfigEvaluationOptions::default(), |o| o.into()),
        )
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_dynamic_config_exposure(
        &self,
        user: &StatsigUserPy,
        name: &str,
    ) -> PyResult<()> {
        self.inner
            .manually_log_dynamic_config_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(name="_INTERNAL_get_experiment", signature = (user, name, options=None))]
    pub fn _internal_get_experiment(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<ExperimentEvaluationOptionsPy>,
        py: Python,
    ) -> String {
        let mut options_actual = options
            .as_ref()
            .map_or(ExperimentEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        self.inner
            .get_raw_experiment_with_options(&user.inner, name, options_actual)
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_experiment_exposure(
        &self,
        user: &StatsigUserPy,
        name: &str,
    ) -> PyResult<()> {
        self.inner
            .manually_log_experiment_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(name="_INTERNAL_get_layer", signature = (user, name, options=None))]
    pub fn _internal_get_layer(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<LayerEvaluationOptionsPy>,
        py: Python,
    ) -> String {
        let mut options_actual = options
            .as_ref()
            .map_or(LayerEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        self.inner
            .get_raw_layer_with_options(&user.inner, name, options_actual)
    }

    #[pyo3(name="_INTERNAL_log_layer_param_exposure", signature = (raw, param_name))]
    pub fn _internal_log_layer_param_exposure(&self, raw: String, param_name: String) {
        self.inner
            .log_layer_param_exposure_from_raw(raw, param_name);
    }

    #[pyo3(signature = (user, name, param_name))]
    pub fn manually_log_layer_parameter_exposure(
        &self,
        user: &StatsigUserPy,
        name: &str,
        param_name: String,
    ) -> PyResult<()> {
        self.inner
            .manually_log_layer_parameter_exposure(&user.inner, name, param_name);
        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_parameter_store(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<ParameterStoreEvaluationOptionsPy>,
    ) -> ParameterStorePy {
        let options_actual =
            options.map_or(ParameterStoreEvaluationOptions::default(), |o| o.into());
        ParameterStorePy {
            name: name.to_string(),
            inner_statsig: Arc::downgrade(&self.inner),
            user: user.inner.clone(),
            options: options_actual,
        }
    }

    #[pyo3(signature = (user, hash=None, client_sdk_key=None, include_local_overrides=None))]
    pub fn get_client_initialize_response(
        &self,
        user: &StatsigUserPy,
        hash: Option<&str>,
        client_sdk_key: Option<&str>,
        include_local_overrides: Option<bool>,
    ) -> String {
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
        opts.include_local_overrides = include_local_overrides;
        self.inner
            .get_client_init_response_with_options_as_string(&user.inner, &opts)
    }

    #[pyo3(signature = (gate_name, value, id=None))]
    pub fn override_gate(&self, gate_name: &str, value: bool, id: Option<&str>) -> PyResult<()> {
        self.inner.override_gate(gate_name, value, id);
        Ok(())
    }

    #[pyo3(signature = (config_name, value, id=None))]
    pub fn override_dynamic_config(
        &self,
        config_name: &str,
        value: Bound<PyDict>,
        id: Option<&str>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_dynamic_config(config_name, value_inner, id);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, value, id=None))]
    pub fn override_experiment(
        &self,
        experiment_name: &str,
        value: Bound<PyDict>,
        id: Option<&str>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_experiment(experiment_name, value_inner, id);
        Ok(())
    }

    #[pyo3(signature = (layer_name, value, id=None))]
    pub fn override_layer(
        &self,
        layer_name: &str,
        value: Bound<PyDict>,
        id: Option<&str>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner.override_layer(layer_name, value_inner, id);
        Ok(())
    }

    #[pyo3(signature = (parameter_store_name, value, id=None))]
    pub fn override_parameter_store(
        &self,
        parameter_store_name: &str,
        value: Bound<PyDict>,
        id: Option<&str>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_parameter_store(parameter_store_name, value_inner, id);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, group_name, id=None))]
    pub fn override_experiment_by_group_name(
        &self,
        experiment_name: &str,
        group_name: &str,
        id: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .override_experiment_by_group_name(experiment_name, group_name, id);
        Ok(())
    }

    #[pyo3(signature = (gate_name, id=None))]
    pub fn remove_gate_override(&self, gate_name: &str, id: Option<&str>) -> PyResult<()> {
        self.inner.remove_gate_override(gate_name, id);
        Ok(())
    }

    #[pyo3(signature = (config_name, id=None))]
    pub fn remove_dynamic_config_override(
        &self,
        config_name: &str,
        id: Option<&str>,
    ) -> PyResult<()> {
        self.inner.remove_dynamic_config_override(config_name, id);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, id=None))]
    pub fn remove_experiment_override(
        &self,
        experiment_name: &str,
        id: Option<&str>,
    ) -> PyResult<()> {
        self.inner.remove_experiment_override(experiment_name, id);
        Ok(())
    }

    #[pyo3(signature = (layer_name, id=None))]
    pub fn remove_layer_override(&self, layer_name: &str, id: Option<&str>) -> PyResult<()> {
        self.inner.remove_layer_override(layer_name, id);
        Ok(())
    }

    #[pyo3(signature = (parameter_store_name, id=None))]
    pub fn remove_parameter_store_override(
        &self,
        parameter_store_name: &str,
        id: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .remove_parameter_store_override(parameter_store_name, id);
        Ok(())
    }

    #[pyo3(signature = ())]
    pub fn remove_all_overrides(&self) -> PyResult<()> {
        self.inner.remove_all_overrides();
        Ok(())
    }

    #[pyo3(name = "get_feature_gate_list")]
    pub fn get_feature_gate_list(&self) -> Vec<String> {
        self.inner.get_feature_gate_list()
    }

    #[pyo3(name = "get_dynamic_config_list")]
    pub fn get_dynamic_config_list(&self) -> Vec<String> {
        self.inner.get_dynamic_config_list()
    }

    #[pyo3(name = "get_experiment_list")]
    pub fn get_experiment_list(&self) -> Vec<String> {
        self.inner.get_experiment_list()
    }

    #[pyo3(name = "get_parameter_store_list")]
    pub fn get_parameter_store_list(&self) -> Vec<String> {
        self.inner.get_parameter_store_list()
    }

    #[pyo3(signature = (user))]
    pub fn identify(&self, user: &StatsigUserPy) -> PyResult<()> {
        self.inner.identify(&user.inner);
        Ok(())
    }
}

fn get_completion_event(py: Python) -> PyResult<(PyObject, PyObject)> {
    let threading = PyModule::import(py, "threading")?;
    let event = threading.call_method0("Event")?;
    let event_clone: PyObject = event.clone().into();

    Ok((event.into(), event_clone))
}

fn create_python_future(py: Python) -> PyResult<(PyObject, PyObject)> {
    let concurrent = PyModule::import(py, "concurrent.futures")?;
    let future = concurrent.getattr("Future")?.call0()?;
    let future_clone: PyObject = future.clone().into();

    Ok((future.into(), future_clone))
}

fn convert_to_number(value: Option<&Bound<PyAny>>) -> Option<f64> {
    let value = value?;

    value.extract::<f64>().ok()
}

fn convert_to_string(value: Option<&Bound<PyAny>>) -> Option<String> {
    let value = value?;

    value.extract::<String>().ok()
}

fn extract_event_metadata(metadata: Option<Bound<PyDict>>) -> Option<HashMap<String, Value>> {
    metadata.map(|m| py_dict_to_json_value_map(&m))
}

fn extract_user_persisted_values(
    py: Python,
    spec_name: &str,
    values: Py<PyDict>,
) -> Option<UserPersistedValues> {
    match convert_dict_to_user_persisted_values(py, values, spec_name) {
        Ok(persisted) => Some(persisted),
        Err(e) => {
            log_e!(
                TAG,
                "Failed to convert persisted values from pydict to rust: {} {:?}",
                spec_name,
                e
            );
            None
        }
    }
}

fn call_completion_event(event: &PyObject, py: Python) {
    if let Err(e) = event.call_method0(py, "set") {
        log_e!(TAG, "Failed to set event: {}", e);
    }
}

fn call_completion_future<'py, A>(future: &PyObject, py: Python<'py>, args: A)
where
    A: IntoPyObject<'py, Target = PyTuple>,
{
    if let Err(e) = future.call_method1(py, "set_result", args) {
        log_e!(TAG, "Failed to set future result: {}", e);
    }
}
