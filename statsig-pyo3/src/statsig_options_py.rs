use pyo3::prelude::*;
use sigstat::StatsigOptions;
use std::sync::Arc;

#[pyclass(name = "StatsigOptions")]
pub struct StatsigOptionsPy {
    pub inner: Arc<StatsigOptions>,
}

#[pymethods]
impl StatsigOptionsPy {
    #[new]
    #[pyo3(signature = (specs_url=None, log_event_url=None, environment=None))]
    pub fn new(specs_url: Option<String>, log_event_url: Option<String>, environment: Option<String>) -> Self {
        let mut options = StatsigOptions::new();
        options.specs_url = specs_url;
        options.log_event_url = log_event_url;
        options.environment = environment;

        Self {
            inner: Arc::new(options),
        }
    }
}
