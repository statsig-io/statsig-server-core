use crate::net_provider_py::NetworkProviderPy;
use crate::pyo_utils::{map_to_py_dict, py_dict_to_json_value_map};
use crate::statsig_options_py::{safe_convert_to_statsig_options, StatsigOptionsPy};
use crate::statsig_persistent_storage_override_adapter_py::convert_dict_to_user_persisted_values;
use crate::statsig_types_py::{DynamicConfigPy, LayerPy};
use crate::{
    statsig_types_py::{
        DynamicConfigEvaluationOptionsPy, ExperimentEvaluationOptionsPy, ExperimentPy,
        FeatureGateEvaluationOptionsPy, FeatureGatePy, LayerEvaluationOptionsPy,
    },
    statsig_user_py::StatsigUserPy,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::*;
use statsig_rust::networking::providers::NetworkProviderGlobal;
use statsig_rust::networking::NetworkProvider;
use statsig_rust::{
    log_e, unwrap_or_return, ClientInitResponseOptions, DynamicConfigEvaluationOptions,
    ExperimentEvaluationOptions, FeatureGateEvaluationOptions, HashAlgorithm,
    LayerEvaluationOptions, ObservabilityClient, Statsig,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const TAG: &str = stringify!(StatsigBasePy);

macro_rules! process_user_persisted_values {
    ($options:expr, $name:expr) => {
        Python::with_gil(|py| match $options.and_then(|o| o.user_persisted_values) {
            Some(user_persisted_value_py) => {
                let a: Result<HashMap<String, statsig_rust::StickyValues>, PyErr> =
                    convert_dict_to_user_persisted_values(py, user_persisted_value_py, $name);
                match a {
                    Ok(a_converted) => Some(a_converted),
                    Err(e) => {
                        log_e!(
                            TAG,
                            "Failed to convert persisted values from pydict to rust: {:?}",
                            e
                        );
                        None
                    }
                }
            }
            None => None,
        })
    };
}

#[gen_stub_pyclass]
#[pyclass(subclass)]
pub struct StatsigBasePy {
    inner: Arc<Statsig>,
    observability_client: Mutex<Option<Arc<dyn ObservabilityClient>>>,
    network_provider: Mutex<Option<Arc<dyn NetworkProvider>>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StatsigBasePy {
    #[new]
    #[pyo3(signature = (network_func, sdk_key, options=None))]
    pub fn new(
        network_func: PyObject,
        sdk_key: &str,
        options: Option<StatsigOptionsPy>,
        py: Python,
    ) -> Self {
        let (opts, ob_client) = safe_convert_to_statsig_options(py, options);

        let network_provider: Arc<dyn NetworkProvider> =
            Arc::new(NetworkProviderPy { network_func });

        NetworkProviderGlobal::set(&network_provider);

        Self {
            inner: Arc::new(Statsig::new(sdk_key, opts.map(Arc::new))),
            observability_client: Mutex::new(ob_client),
            network_provider: Mutex::new(Some(network_provider)),
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
        let network_provider = match self.network_provider.lock() {
            Ok(mut lock) => lock.take(),
            _ => None,
        };
        let obs_client = match self.observability_client.lock() {
            Ok(mut lock) => lock.take(),
            _ => None,
        };

        rt.get_handle().spawn(async move {
            if let Err(e) = inst.shutdown().await {
                log_e!(TAG, "Failed to gracefully shutdown StatsigPy: {}", e);
            }

            Python::with_gil(|py| {
                if let Err(e) = event_clone.call_method0(py, "set") {
                    log_e!(TAG, "Failed to set event: {}", e);
                }
            });

            // held until the shutdown is complete
            drop(network_provider);
            drop(obs_client);
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
        options_actual.user_persisted_values = process_user_persisted_values!(options, name);

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
        options_actual.user_persisted_values = process_user_persisted_values!(options, name);

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

    #[pyo3(signature = (gate_name, value))]
    pub fn override_gate(&self, gate_name: &str, value: bool) -> PyResult<()> {
        self.inner.override_gate(gate_name, value);
        Ok(())
    }

    #[pyo3(signature = (config_name, value))]
    pub fn override_dynamic_config(&self, config_name: &str, value: Bound<PyDict>) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner.override_dynamic_config(config_name, value_inner);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, value))]
    pub fn override_experiment(&self, experiment_name: &str, value: Bound<PyDict>) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner.override_experiment(experiment_name, value_inner);
        Ok(())
    }

    #[pyo3(signature = (layer_name, value))]
    pub fn override_layer(&self, layer_name: &str, value: Bound<PyDict>) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner.override_layer(layer_name, value_inner);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, group_name))]
    pub fn override_experiment_by_group_name(
        &self,
        experiment_name: &str,
        group_name: &str,
    ) -> PyResult<()> {
        self.inner
            .override_experiment_by_group_name(experiment_name, group_name);
        Ok(())
    }

    #[pyo3(signature = (gate_name, for_id, value))]
    pub fn override_gate_for_id(&self, gate_name: &str, for_id: &str, value: bool) -> PyResult<()> {
        self.inner.override_gate_for_id(gate_name, for_id, value);
        Ok(())
    }

    #[pyo3(signature = (config_name, for_id, value))]
    pub fn override_dynamic_config_for_id(
        &self,
        config_name: &str,
        for_id: &str,
        value: Bound<PyDict>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_dynamic_config_for_id(config_name, for_id, value_inner);
        Ok(())
    }

    #[pyo3(signature = (experiment_name,for_id, value))]
    pub fn override_experiment_for_id(
        &self,
        experiment_name: &str,
        for_id: &str,
        value: Bound<PyDict>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_experiment_for_id(experiment_name, for_id, value_inner);
        Ok(())
    }

    #[pyo3(signature = (layer_name, for_id, value))]
    pub fn override_layer_for_id(
        &self,
        layer_name: &str,
        for_id: &str,
        value: Bound<PyDict>,
    ) -> PyResult<()> {
        let value_inner = py_dict_to_json_value_map(&value);
        self.inner
            .override_layer_for_id(layer_name, for_id, value_inner);
        Ok(())
    }

    #[pyo3(signature = (experiment_name, for_id, group_name))]
    pub fn override_experiment_by_group_name_for_id(
        &self,
        experiment_name: &str,
        for_id: &str,
        group_name: &str,
    ) -> PyResult<()> {
        self.inner
            .override_experiment_by_group_name_for_id(experiment_name, for_id, group_name);
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
