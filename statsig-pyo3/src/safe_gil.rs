use pyo3::{prelude::*, Python};
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use statsig_rust::log_w;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static::lazy_static! {
    static ref PYTHON_RUNNING: AtomicBool = AtomicBool::new(true);
}

#[gen_stub_pyfunction]
#[pyfunction]
pub fn notify_python_shutdown() {
    PYTHON_RUNNING.store(false, Ordering::SeqCst);
}

const TAG: &str = "SafeGil";

pub struct SafeGil;

impl SafeGil {
    pub fn run<F, R>(f: F) -> R
    where
        F: for<'py> FnOnce(Option<Python<'py>>) -> R,
    {
        if !PYTHON_RUNNING.load(Ordering::SeqCst) {
            log_w!(TAG, "GIL Unavailable: Python interpreter is shutting down");

            return f(None);
        }

        Python::with_gil(move |py| f(Some(py)))
    }
}
