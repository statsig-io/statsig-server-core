use pyo3::{pyclass, pymethods, FromPyObject, PyObject};
use pyo3_stub_gen::derive::*;
use statsig_rust::output_logger::OutputLogProvider;

use crate::pyo_utils::{call_py_function_with_no_args, call_py_function_with_two_args};

#[gen_stub_pyclass]
#[pyclass(name = "OutputLoggerProviderBase", subclass)]
#[derive(FromPyObject, Default)]
pub struct OutputLoggerProviderBasePy {
    pub init_fn: Option<PyObject>,
    pub debug_fn: Option<PyObject>,
    pub info_fn: Option<PyObject>,
    pub warn_fn: Option<PyObject>,
    pub error_fn: Option<PyObject>,
    pub shutdown_fn: Option<PyObject>,
}

#[gen_stub_pymethods]
#[pymethods]
impl OutputLoggerProviderBasePy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

impl OutputLogProvider for OutputLoggerProviderBasePy {
    fn initialize(&self) {
        call_py_function_with_no_args("initialize", &self.init_fn);
    }

    fn debug(&self, tag: &str, msg: String) {
        call_py_function_with_two_args("debug", &self.debug_fn, tag, &msg);
    }

    fn info(&self, tag: &str, msg: String) {
        call_py_function_with_two_args("info", &self.info_fn, tag, &msg);
    }

    fn warn(&self, tag: &str, msg: String) {
        call_py_function_with_two_args("warn", &self.warn_fn, tag, &msg);
    }

    fn error(&self, tag: &str, msg: String) {
        call_py_function_with_two_args("error", &self.error_fn, tag, &msg);
    }

    fn shutdown(&self) {
        call_py_function_with_no_args("shutdown", &self.shutdown_fn);
    }
}
