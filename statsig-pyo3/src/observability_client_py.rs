use crate::pyo_utils::string_map_to_py_dict;
use pyo3::{pyclass, pymethods, Py, PyResult, Python};
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_e, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::Arc;

const TAG: &str = "ObservabilityClientPy";

#[gen_stub_pyclass]
#[pyclass(name = "ObservabilityClient", subclass)]
pub struct ObservabilityClientPy {
    #[pyo3(get)]
    py_ref: Option<Py<ObservabilityClientPy>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ObservabilityClientPy {
    #[new]
    pub fn new(_py: Python) -> PyResult<Self> {
        let instance = ObservabilityClientPy { py_ref: None };
        Ok(instance)
    }

    #[pyo3(name = "set_py_ref")]
    fn set_py_ref(&mut self, py_ref: Py<ObservabilityClientPy>) {
        self.py_ref = Some(py_ref);
    }

    #[pyo3(name = "init")]
    fn init_py(&self) {
        self.call_python_method("init");
    }

    #[pyo3(name = "increment", signature = (metric_name, value, tags=None))]
    fn increment_py(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.call_python_method_with_args("increment", (metric_name, value, tags));
    }

    #[pyo3(name = "gauge", signature = (metric_name, value, tags=None))]
    fn gauge_py(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.call_python_method_with_args("gauge", (metric_name, value, tags));
    }

    #[pyo3(name = "distribution", signature = (metric_name, value, tags=None))]
    fn dist_py(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.call_python_method_with_args("distribution", (metric_name, value, tags));
    }
}

impl ObservabilityClientPy {
    fn call_python_method(&self, method_name: &str) {
        Python::with_gil(|py| {
            let Some(py_ref) = &self.py_ref else {
                log_e!(TAG, "No Python reference stored in `py_ref`");
                return;
            };

            let Ok(method) = py_ref.getattr(py, method_name) else {
                log_e!(TAG, "Python subclass does not override `{}`", method_name);
                return;
            };

            if let Err(e) = method.call0(py) {
                log_e!(
                    TAG,
                    "Failed to call Python subclass `{}`: {}",
                    method_name,
                    e
                );
            }
        });
    }

    fn call_python_method_with_args(
        &self,
        method_name: &str,
        args: (String, f64, Option<HashMap<String, String>>),
    ) {
        Python::with_gil(|py| {
            let Some(py_ref) = &self.py_ref else {
                log_e!(TAG, "No Python reference stored in `py_ref`");
                return;
            };

            let Ok(method) = py_ref.getattr(py, method_name) else {
                log_e!(TAG, "Python subclass does not override `{}`", method_name);
                return;
            };

            let (metric_name, value, tags) = args;
            let py_tags = tags.map(|t| string_map_to_py_dict(py, &t));

            if let Err(e) = method.call1(py, (metric_name, value, py_tags)) {
                log_e!(
                    TAG,
                    "Failed to call Python subclass `{}`: {}",
                    method_name,
                    e
                );
            }
        });
    }
}

impl Clone for ObservabilityClientPy {
    fn clone(&self) -> Self {
        Python::with_gil(|py| ObservabilityClientPy {
            py_ref: self.py_ref.as_ref().map(|r| r.clone_ref(py)),
        })
    }
}

impl ObservabilityClient for ObservabilityClientPy {
    fn init(&self) {
        self.init_py();
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.increment_py(metric_name, value, tags);
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.gauge_py(metric_name, value, tags);
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.dist_py(metric_name, value, tags);
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}
