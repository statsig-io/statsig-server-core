use crate::pyo_utils::{map_to_py_dict, py_dict_to_json_value_map};
use crate::safe_gil::SafeGil;
use crate::statsig_options_py::{safe_convert_to_statsig_options, StatsigOptionsPy};
use crate::statsig_persistent_storage_override_adapter_py::convert_dict_to_user_persisted_values;
use crate::statsig_types_py::{
    DynamicConfigPy, InitializeDetailsPy, LayerPy, ParameterStoreEvaluationOptionsPy,
    ParameterStorePy,
};
use crate::{
    statsig_types_py::{
        DynamicConfigEvaluationOptionsPy, ExperimentEvaluationOptionsPy, ExperimentPy,
        FeatureGateEvaluationOptionsPy, FeatureGatePy, LayerEvaluationOptionsPy,
    },
    statsig_user_py::StatsigUserPy,
};
use parking_lot::Mutex;
use pyo3::types::PyTuple;
use pyo3::{prelude::*, types::PyDict};
use pyo3_stub_gen::derive::*;
use statsig_rust::{
    log_e, unwrap_or_return, ClientInitResponseOptions, DynamicConfigEvaluationOptions,
    ExperimentEvaluationOptions, FeatureGateEvaluationOptions, HashAlgorithm,
    LayerEvaluationOptions, ObservabilityClient, ParameterStoreEvaluationOptions, Statsig,
    UserPersistedValues,
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

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_feature_gate(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<FeatureGateEvaluationOptionsPy>,
    ) -> FeatureGatePy {
        let gate = self.inner.get_feature_gate_with_options(
            &user.inner,
            name,
            options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()),
        );
        FeatureGatePy {
            name: gate.name,
            value: gate.value,
            rule_id: gate.rule_id,
            id_type: gate.id_type,
            details: gate.details.into(),
        }
    }

    #[pyo3(signature = (user, name))]
    pub fn manually_log_gate_exposure(&self, user: &StatsigUserPy, name: &str) -> PyResult<()> {
        self.inner.manually_log_gate_exposure(&user.inner, name);
        Ok(())
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_dynamic_config(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<DynamicConfigEvaluationOptionsPy>,
        py: Python,
    ) -> DynamicConfigPy {
        let config = self.inner.get_dynamic_config_with_options(
            &user.inner,
            name,
            options.map_or(DynamicConfigEvaluationOptions::default(), |o| o.into()),
        );

        DynamicConfigPy {
            name: config.name.clone(),
            rule_id: config.rule_id.clone(),
            id_type: config.id_type.clone(),
            value: map_to_py_dict(py, &config.value),
            details: config.details.clone().into(),
            inner: config,
        }
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

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_experiment(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<ExperimentEvaluationOptionsPy>,
        py: Python,
    ) -> ExperimentPy {
        let mut options_actual = options
            .as_ref()
            .map_or(ExperimentEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        let experiment = self
            .inner
            .get_experiment_with_options(&user.inner, name, options_actual);

        ExperimentPy {
            name: experiment.name.clone(),
            rule_id: experiment.rule_id.clone(),
            id_type: experiment.id_type.clone(),
            group_name: experiment.group_name.clone(),
            value: map_to_py_dict(py, &experiment.value),
            details: experiment.details.clone().into(),
            inner: experiment,
        }
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

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_layer(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<LayerEvaluationOptionsPy>,
        py: Python,
    ) -> LayerPy {
        let mut options_actual = options
            .as_ref()
            .map_or(LayerEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        let layer = self
            .inner
            .get_layer_with_options(&user.inner, name, options_actual);

        LayerPy {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            allocated_experiment_name: layer.allocated_experiment_name.clone(),
            value: map_to_py_dict(py, &layer.__value),
            details: layer.details.clone().into(),
            inner: layer,
        }
    }

    #[pyo3(signature = (user, name, options=None))]
    pub fn get_prompt(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<LayerEvaluationOptionsPy>,
        py: Python,
    ) -> LayerPy {
        let mut options_actual = options
            .as_ref()
            .map_or(LayerEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        let layer = self
            .inner
            .get_prompt_with_options(&user.inner, name, options_actual);

        LayerPy {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            allocated_experiment_name: layer.allocated_experiment_name.clone(),
            value: map_to_py_dict(py, &layer.__value),
            details: layer.details.clone().into(),
            inner: layer,
        }
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
