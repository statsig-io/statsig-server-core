use pyo3::prelude::*;
use sigstat::statsig_types::Experiment;

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

    pub inner: Experiment,
}

#[pymethods]
impl ExperimentPy {
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
}
