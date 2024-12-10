use std::sync::Arc;
use crate::statsig_user_py::StatsigUserPy;
use sigstat::{log_d, Statsig, StatsigRuntime};
use pyo3::{prelude::*, wrap_pyfunction};

const TAG: &str = stringify!(StatsigPy);

#[pyclass(eq, eq_int)]
#[derive(PartialEq)]
enum StatsigResultPy {
    Ok,
    InvalidKey,
    NoDice,
}

#[pyclass]
pub struct StatsigPy {
    inner: Arc<Statsig>,
}

#[pymethods]
impl StatsigPy {
    #[new]
    pub fn new(sdk_key: &str) -> Self {
        Self {
            inner: Arc::new(Statsig::new(sdk_key, None)),
        }
    }

    pub fn initialize(&self) -> StatsigResultPy {
        println!("Init Now...");
        let statsig_rt = StatsigRuntime::get_runtime();
        let me = self.inner.clone();
        let result = statsig_rt.runtime_handle.block_on(async move {
            me.initialize().await
        });

        println!("Init End");

        match result {
            Ok(_) => StatsigResultPy::Ok,
            Err(_) => StatsigResultPy::NoDice,
        }
    }

    pub fn check_gate(self_: PyRef<'_, Self>, name: &str, user: PyRef<'_, StatsigUserPy>) -> bool {
        self_.inner.check_gate(&user.inner, name)
    }
}
