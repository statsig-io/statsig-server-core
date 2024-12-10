use pyo3::prelude::*;
use sigstat::StatsigUser;

#[pyclass]
pub struct StatsigUserPy {
    pub inner: StatsigUser,
}

#[pymethods]
impl StatsigUserPy {
    #[new]
    pub fn new(user_id: &str) -> Self {
        Self {
            inner: StatsigUser::with_user_id(user_id.to_string()),
        }
    }
}
