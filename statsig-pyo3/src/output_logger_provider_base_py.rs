use pyo3::{pyclass, pymethods, FromPyObject, PyObject};
use pyo3_stub_gen::derive::*;
use statsig_rust::output_logger::OutputLogProvider;

use crate::safe_gil::SafeGil;

const TAG: &str = "OutputLoggerProviderBasePy";

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

macro_rules! log_stdout {
    // Intentionaly log directly to stdout to avoid circular dependency
    ($tag:expr, $($arg:tt)*) => {
        {
            println!("{}: {}", $tag, format!($($arg)*));
        }
    }
}

fn call_py_function_with_no_args(func_name: &str, func_opt: &Option<PyObject>) {
    if func_opt.is_none() {
        log_stdout!(TAG, "No function to call for {}", func_name);
        return;
    }

    SafeGil::run(|py| {
        let py = match py {
            Some(p) => p,
            None => return,
        };

        let func = match func_opt.as_ref() {
            Some(f) => f,
            None => {
                log_stdout!(TAG, "Function is None for {}", func_name);
                return;
            }
        };

        let result = func.call(py, (), None);
        if let Err(e) = result {
            log_stdout!(
                TAG,
                "Failed to call OutputLoggerProvider.{}: {:?}",
                func_name,
                e
            );
        }
    });
}

fn call_py_function_with_two_args(
    func_name: &str,
    func_opt: &Option<PyObject>,
    arg1: &str,
    arg2: &str,
) {
    let func = match func_opt {
        Some(f) => f,
        None => {
            log_stdout!(TAG, "No function to call for {}", func_name);
            return;
        }
    };

    SafeGil::run(|py| {
        let py = match py {
            Some(p) => p,
            None => return,
        };

        let result = func.call(py, (arg1, arg2), None);
        if let Err(e) = result {
            log_stdout!(
                TAG,
                "Failed to call OutputLoggerProvider.{}: {:?}",
                func_name,
                e
            );
        }
    });
}
