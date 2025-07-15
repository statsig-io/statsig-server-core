use pyo3::{prelude::*, Python};
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use statsig_rust::statsig_global::StatsigGlobal;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static::lazy_static! {
    static ref PYTHON_RUNNING: AtomicBool = AtomicBool::new(true);
}

#[gen_stub_pyfunction]
#[pyfunction]
pub fn notify_python_shutdown() {
    PYTHON_RUNNING.store(false, Ordering::SeqCst);
}

#[gen_stub_pyfunction]
#[pyfunction]
pub fn notify_python_fork() {
    StatsigGlobal::reset();
}

pub struct SafeGil;

impl SafeGil {
    pub fn run<F, R>(f: F) -> R
    where
        F: for<'py> FnOnce(Option<Python<'py>>) -> R,
    {
        if !PYTHON_RUNNING.load(Ordering::SeqCst) {
            log_gil_unavailable();

            return f(None);
        }

        Python::with_gil(move |py| f(Some(py)))
    }
}

fn log_gil_unavailable() {
    // we do not use the logging framework here as it may recursively try access the GIL
    eprintln!("StatsigSafeGil: GIL Unavailable. Python interpreter is shutting down");
}
