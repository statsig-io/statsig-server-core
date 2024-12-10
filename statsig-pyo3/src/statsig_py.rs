use crate::{
    evaluation_types_py::{ExperimentPy, FeatureGatePy},
    statsig_user_py::StatsigUserPy,
};
use pyo3::prelude::*;
use sigstat::{log_d, ClientInitResponseOptions, Statsig, StatsigRuntime};
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
    pub fn new(sdk_key: &str) -> Self {
        Self {
            inner: Arc::new(Statsig::new(sdk_key, None)),
        }
    }

    pub fn initialize(&self) -> StatsigResultPy {
        log_d!(TAG, "Initializing...");
        let statsig_rt = StatsigRuntime::get_runtime();
        let me = self.inner.clone();
        let result = statsig_rt
            .runtime_handle
            .block_on(async move { me.initialize().await });

        log_d!(TAG, "Initialization Done: {:?}", result);

        match result {
            Ok(_) => StatsigResultPy::Ok,
            Err(_) => StatsigResultPy::NoDice,
        }
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
