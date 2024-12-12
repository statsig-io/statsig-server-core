use std::sync::Arc;
use pyo3::prelude::*;
use sigstat::{StatsigOptions};

#[pyclass(name="StatsigOptions")]
pub struct StatsigOptionsPy {
    pub inner: Arc<StatsigOptions>,
}

#[pymethods]
impl StatsigOptionsPy {
    #[new]
    #[pyo3(signature = (specs_url=None))]
    pub fn new(specs_url: Option<String>) -> Self {
        let mut options = StatsigOptions::new();
        options.specs_url = specs_url;

        Self {
            inner: Arc::new(options),
        }
    }
}
