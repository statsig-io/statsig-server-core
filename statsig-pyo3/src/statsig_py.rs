use crate::statsig_options_py::StatsigOptionsPy;
use crate::{
    evaluation_types_py::{ExperimentPy, FeatureGatePy},
    statsig_user_py::StatsigUserPy,
};
use pyo3::prelude::*;
use sigstat::{log_e, ClientInitResponseOptions, Statsig};
use std::sync::Arc;

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
        let threading = PyModule::import(py, "threading")?;
        let completion_event = threading.call_method0("Event")?;
        let event_clone: PyObject = completion_event.clone().into();

        let inst = self.inner.clone();
        self.inner.statsig_runtime.runtime_handle.spawn(async move {
            if let Err(e) = inst.initialize().await {
                log_e!(TAG, "Failed to initialize Statsig: {}", e);
            }

            Python::with_gil(|py| {
                event_clone.call_method0(py, "set").unwrap();
            });
        });

        Ok(completion_event.into())
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

    pub fn get_experiment(&self, name: &str, user: &StatsigUserPy) -> ExperimentPy {
        let experiment = self.inner.get_experiment(&user.inner, name);

        ExperimentPy {
            name: experiment.name,
            value: experiment.value,
            rule_id: experiment.rule_id,
            id_type: experiment.id_type,
            group_name: experiment.group_name,
        }
    }

    pub fn get_client_init_response(&self, user: &StatsigUserPy) -> String {
        let opts = ClientInitResponseOptions::default();
        self.inner
            .get_client_init_response_with_options_as_string(&user.inner, &opts)
    }
}
