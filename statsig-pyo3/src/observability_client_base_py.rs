use pyo3::{pyclass, pymethods, FromPyObject, PyObject};
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_e, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::Arc;

use crate::safe_gil::SafeGil;

const TAG: &str = "ObservabilityClientBasePy";

#[gen_stub_pyclass]
#[pyclass(
    name = "ObservabilityClientBase",
    module = "statsig_python_core",
    subclass
)]
#[derive(FromPyObject, Default)]
pub struct ObservabilityClientBasePy {
    init_fn: Option<PyObject>,
    increment_fn: Option<PyObject>,
    gauge_fn: Option<PyObject>,
    dist_fn: Option<PyObject>,
    error_fn: Option<PyObject>,
    should_enable_high_cardinality_for_this_tag_fn: Option<PyObject>,
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
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            if let Some(init_fn) = &self.init_fn {
                if let Err(e) = init_fn.call(py, (), None) {
                    log_e!(TAG, "Failed to call ObservabilityClient.init: {:?}", e);
                }
            }
        });
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            if let Some(func) = &self.increment_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.increment: {:?}", e);
                }
            }
        });
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            if let Some(func) = &self.gauge_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.gauge: {:?}", e);
                }
            }
        });
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            if let Some(func) = &self.dist_fn {
                let args = (metric_name, value, tags);
                if let Err(e) = func.call1(py, args) {
                    log_e!(TAG, "Failed to call ObservabilityClient.dist: {:?}", e);
                }
            }
        });
    }

    fn error(&self, tag: String, error: String) {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            let error_fn = match &self.error_fn {
                Some(func) => func,
                None => return,
            };

            if let Err(e) = error_fn.call1(py, (tag, error)) {
                log_e!(
                    TAG,
                    "Failed to call ObservabilityClient.error_callback: {:?}",
                    e
                );
            };
        });
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }

    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool> {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return None,
            };

            let func = match &self.should_enable_high_cardinality_for_this_tag_fn {
                Some(f) => f,
                None => return None,
            };

            let value = match func.call1(py, (tag,)) {
                Ok(value) => value,
                Err(e) => {
                    log_e!(TAG, "Failed to call ObservabilityClient.should_enable_high_cardinality_for_this_tag: {:?}", e);
                    return None;
                }
            };

            match value.extract(py) {
                Ok(value) => Some(value),
                Err(e) => {
                    log_e!(TAG, "Failed to extract ObservabilityClient.should_enable_high_cardinality_for_this_tag: {:?}", e);
                    None
                }
            }
        })
    }
}
