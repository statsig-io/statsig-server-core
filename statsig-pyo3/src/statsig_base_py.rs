use crate::pyo_utils::{map_to_py_dict_direct, py_dict_to_json_value_map};
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
use pyo3::exceptions::PyRuntimeError;
use pyo3::{call::PyCallArgs, prelude::*, types::PyDict};
use pyo3_stub_gen::derive::*;
use serde::Serialize;
use serde_json::Value;
use statsig_rust::evaluation::evaluation_details::EvaluationDetails;
use statsig_rust::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
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

    pub fn initialize(&self, py: Python) -> PyResult<Py<PyAny>> {
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

    pub fn initialize_with_details(&self, py: Python) -> PyResult<Py<PyAny>> {
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

    pub fn flush_events(&self, py: Python) -> PyResult<Py<PyAny>> {
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

    pub fn shutdown(&self, py: Python) -> PyResult<Py<PyAny>> {
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

    #[pyo3(name="_INTERNAL_get_feature_gate_as_dict", signature = (user, name, options=None))]
    pub fn _internal_get_feature_gate_as_dict(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<FeatureGateEvaluationOptionsPy>,
        py: Python,
    ) -> PyResult<Py<PyDict>> {
        let gate = self.inner.get_feature_gate_with_options(
            &user.inner,
            name,
            options.map_or(FeatureGateEvaluationOptions::default(), |o| o.into()),
        );

        feature_gate_to_py_dict(py, &gate)
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

    #[pyo3(name="_INTERNAL_get_dynamic_config_as_dict", signature = (user, name, options=None))]
    pub fn _internal_get_dynamic_config_as_dict(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<DynamicConfigEvaluationOptionsPy>,
        py: Python,
    ) -> PyResult<Py<PyDict>> {
        let config = self.inner.get_dynamic_config_with_options(
            &user.inner,
            name,
            options.map_or(DynamicConfigEvaluationOptions::default(), |o| o.into()),
        );

        dynamic_config_to_py_dict(py, &config)
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

    #[pyo3(name="_INTERNAL_get_experiment_as_dict", signature = (user, name, options=None))]
    pub fn _internal_get_experiment_as_dict(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<ExperimentEvaluationOptionsPy>,
        py: Python,
    ) -> PyResult<Py<PyDict>> {
        let mut options_actual = options
            .as_ref()
            .map_or(ExperimentEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        let experiment = self
            .inner
            .get_experiment_with_options(&user.inner, name, options_actual);

        experiment_to_py_dict(py, &experiment)
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

    #[pyo3(
        name="_INTERNAL_get_layer_exposure_raw_and_dict",
        signature = (user, name, options=None)
    )]
    pub fn _internal_get_layer_exposure_raw_and_dict(
        &self,
        user: &StatsigUserPy,
        name: &str,
        options: Option<LayerEvaluationOptionsPy>,
        py: Python,
    ) -> PyResult<(String, Py<PyDict>)> {
        let mut options_actual = options
            .as_ref()
            .map_or(LayerEvaluationOptions::default(), |o| o.into());

        options_actual.user_persisted_values = options
            .and_then(|o| o.user_persisted_values)
            .and_then(|v| extract_user_persisted_values(py, name, v));

        let layer = self
            .inner
            .get_layer_with_options(&user.inner, name, options_actual);

        Ok((layer_to_exposure_raw(&layer)?, layer_to_py_dict(py, &layer)?))
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

fn get_completion_event(py: Python) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
    let threading = PyModule::import(py, "threading")?;
    let event = threading.call_method0("Event")?;
    let event_clone: Py<PyAny> = event.clone().unbind();

    Ok((event.unbind(), event_clone))
}

fn create_python_future(py: Python) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
    let concurrent = PyModule::import(py, "concurrent.futures")?;
    let future = concurrent.getattr("Future")?.call0()?;
    let future_clone: Py<PyAny> = future.clone().unbind();

    Ok((future.unbind(), future_clone))
}

fn convert_to_number(value: Option<&Bound<PyAny>>) -> Option<f64> {
    let value = value?;

    value.extract::<f64>().ok()
}

fn convert_to_string(value: Option<&Bound<PyAny>>) -> Option<String> {
    let value = value?;

    value.extract::<String>().ok()
}

fn feature_gate_to_py_dict(py: Python, gate: &FeatureGate) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("name", &gate.name)?;
    raw.set_item("value", gate.value)?;
    raw.set_item("ruleID", &gate.rule_id)?;
    raw.set_item("idType", &gate.id_type)?;
    raw.set_item("details", evaluation_details_to_py_dict(py, &gate.details)?)?;

    Ok(raw.unbind())
}

fn dynamic_config_to_py_dict(py: Python, config: &DynamicConfig) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("name", &config.name)?;
    raw.set_item("value", map_to_py_dict_direct(py, &config.value)?)?;
    raw.set_item("ruleID", &config.rule_id)?;
    raw.set_item("idType", &config.id_type)?;
    raw.set_item(
        "details",
        evaluation_details_to_py_dict(py, &config.details)?,
    )?;

    Ok(raw.unbind())
}

fn experiment_to_py_dict(py: Python, experiment: &Experiment) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("name", &experiment.name)?;
    raw.set_item("value", map_to_py_dict_direct(py, &experiment.value)?)?;
    raw.set_item("ruleID", &experiment.rule_id)?;
    raw.set_item("idType", &experiment.id_type)?;
    raw.set_item("groupName", &experiment.group_name)?;
    raw.set_item(
        "isExperimentActive",
        experiment.is_experiment_active,
    )?;
    raw.set_item(
        "details",
        evaluation_details_to_py_dict(py, &experiment.details)?,
    )?;

    Ok(raw.unbind())
}

fn layer_to_py_dict(py: Python, layer: &Layer) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("name", &layer.name)?;
    raw.set_item("value", map_to_py_dict_direct(py, &layer.__value)?)?;
    raw.set_item("ruleID", &layer.rule_id)?;
    raw.set_item("idType", &layer.id_type)?;
    raw.set_item("groupName", &layer.group_name)?;
    raw.set_item(
        "isExperimentActive",
        layer.is_experiment_active,
    )?;
    raw.set_item(
        "allocatedExperimentName",
        &layer.allocated_experiment_name,
    )?;
    raw.set_item("details", evaluation_details_to_py_dict(py, &layer.details)?)?;

    Ok(raw.unbind())
}

fn layer_to_exposure_raw(layer: &Layer) -> PyResult<String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LayerExposureRaw<'a> {
        name: &'a str,
        #[serde(rename = "ruleID")]
        rule_id: &'a str,
        id_type: Option<&'a str>,
        group_name: Option<&'a str>,
        details: &'a EvaluationDetails,
        allocated_experiment_name: Option<&'a str>,
        disable_exposure: bool,
        user: &'a statsig_rust::user::StatsigUserLoggable,
        secondary_exposures: Option<&'a Vec<statsig_rust::SecondaryExposure>>,
        undelegated_secondary_exposures: Option<&'a Vec<statsig_rust::SecondaryExposure>>,
        explicit_parameters: Option<&'a statsig_rust::specs_response::explicit_params::ExplicitParameters>,
        parameter_rule_ids:
            Option<&'a std::collections::HashMap<statsig_rust::interned_string::InternedString, statsig_rust::interned_string::InternedString>>,
    }

    let evaluation = layer.__evaluation.as_ref();
    let raw = LayerExposureRaw {
        name: &layer.name,
        rule_id: &layer.rule_id,
        id_type: (!layer.id_type.is_empty()).then_some(layer.id_type.as_str()),
        group_name: layer.group_name.as_deref(),
        details: &layer.details,
        allocated_experiment_name: layer.allocated_experiment_name.as_deref(),
        disable_exposure: layer.__disable_exposure,
        user: &layer.__user,
        secondary_exposures: evaluation.map(|value| &value.base.secondary_exposures),
        undelegated_secondary_exposures: evaluation
            .and_then(|value| value.undelegated_secondary_exposures.as_ref()),
        explicit_parameters: evaluation.map(|value| &value.explicit_parameters),
        parameter_rule_ids: layer.__parameter_rule_ids.as_ref(),
    };

    serde_json::to_string(&raw).map_err(|error| PyRuntimeError::new_err(error.to_string()))
}

fn evaluation_details_to_py_dict(py: Python, details: &EvaluationDetails) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("reason", &details.reason)?;
    raw.set_item("lcut", details.lcut)?;
    raw.set_item("received_at", details.received_at)?;
    raw.set_item("version", details.version)?;

    Ok(raw.unbind())
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

fn call_completion_event(event: &Py<PyAny>, py: Python) {
    if let Err(e) = event.as_ref().call_method0(py, "set") {
        log_e!(TAG, "Failed to set event: {}", e);
    }
}

fn call_completion_future<'py, A>(future: &Py<PyAny>, py: Python<'py>, args: A)
where
    A: PyCallArgs<'py>,
{
    if let Err(e) = future.as_ref().call_method1(py, "set_result", args) {
        log_e!(TAG, "Failed to set future result: {}", e);
    }
}
