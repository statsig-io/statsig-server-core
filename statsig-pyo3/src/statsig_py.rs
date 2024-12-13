use crate::statsig_options_py::StatsigOptionsPy;
use crate::statsig_types_py::{DynamicConfigPy, LayerPy};
use crate::{
    statsig_types_py::{ExperimentPy, FeatureGatePy},
    statsig_user_py::StatsigUserPy,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sigstat::{log_e, unwrap_or_return, ClientInitResponseOptions, Statsig};
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
            local_opts = Some(o.inner.clone());
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

    pub fn check_gate(&self, name: &str, user: &StatsigUserPy) -> bool {
        self.inner.check_gate(&user.inner, name)
    }

    pub fn get_feature_gate(&self, name: &str, user: &StatsigUserPy) -> FeatureGatePy {
        let gate = self.inner.get_feature_gate(&user.inner, name);
        FeatureGatePy {
            name: gate.name,
            value: gate.value,
            rule_id: gate.rule_id,
            id_type: gate.id_type,
        }
    }

    pub fn get_dynamic_config(&self, name: &str, user: &StatsigUserPy) -> DynamicConfigPy {
        let config = self.inner.get_dynamic_config(&user.inner, name);

        DynamicConfigPy {
            name: config.name.clone(),
            rule_id: config.rule_id.clone(),
            id_type: config.id_type.clone(),
            inner: config,
        }
    }

    pub fn get_experiment(&self, name: &str, user: &StatsigUserPy) -> ExperimentPy {
        let experiment = self.inner.get_experiment(&user.inner, name);

        ExperimentPy {
            name: experiment.name.clone(),
            rule_id: experiment.rule_id.clone(),
            id_type: experiment.id_type.clone(),
            group_name: experiment.group_name.clone(),
            inner: experiment,
        }
    }

    pub fn get_layer(&self, name: &str, user: &StatsigUserPy) -> LayerPy {
        let layer = self.inner.get_layer(&user.inner, name);

        LayerPy {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            inner: layer,
        }
    }

    pub fn get_client_init_response(&self, user: &StatsigUserPy) -> String {
        let opts = ClientInitResponseOptions::default();
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
    let value = match value {
        Some(v) => v,
        None => return None,
    };

    value.extract::<f64>().ok()
}

fn convert_to_string(value: Option<&Bound<PyAny>>) -> Option<String> {
    let value = match value {
        Some(v) => v,
        None => return None,
    };

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
