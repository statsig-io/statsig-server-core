use pyo3::{pyclass, pymethods, FromPyObject, PyObject, Python};
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_e, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::Arc;

const TAG: &str = "ObservabilityClientBasePy";

#[gen_stub_pyclass]
#[pyclass(name = "ObservabilityClientBase", subclass)]
#[derive(FromPyObject, Default)]
pub struct ObservabilityClientBasePy {
    init_fn: Option<PyObject>,
    increment_fn: Option<PyObject>,
    gauge_fn: Option<PyObject>,
    dist_fn: Option<PyObject>,
    error_fn: Option<PyObject>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ObservabilityClientBasePy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ObservabilityClient for ObservabilityClientBasePy {
    fn init(&self) {
        Python::with_gil(|py| {
            if let Some(init_fn) = &self.init_fn {
                if let Err(e) = init_fn.call(py, (), None) {
                    log_e!(TAG, "Failed to call ObservabilityClient.init: {:?}", e);
                }
            }
        });
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        Python::with_gil(|py| {
            if let Some(func) = &self.increment_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.increment: {:?}", e);
                }
            }
        });
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        Python::with_gil(|py| {
            if let Some(func) = &self.gauge_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.gauge: {:?}", e);
                }
            }
        });
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        Python::with_gil(|py| {
            if let Some(func) = &self.dist_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.dist: {:?}", e);
                }
            }
        });
    }

    fn error(&self, tag: String, error: String) {
        Python::with_gil(|py| {
            if let Some(func) = &self.error_fn {
                let args = (tag, error);
                if let Err(e) = func.call1(py, args) {
                    log_e!(
                        TAG,
                        "Failed to call ObservabilityClient.error_callback: {:?}",
                        e
                    );
                }
            }
        });
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}
