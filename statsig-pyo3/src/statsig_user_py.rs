use crate::pyo_utils::py_object_to_map;
use pyo3::prelude::*;
use sigstat::{DynamicValue, StatsigUser};

#[pyclass(name = "StatsigUser")]
pub struct StatsigUserPy {
    pub inner: StatsigUser,

    #[pyo3(get, set)]
    pub user_id: String,
    #[pyo3(get, set)]
    pub email: Option<String>,
    #[pyo3(get, set)]
    pub ip: Option<String>,
    #[pyo3(get, set)]
    pub country: Option<String>,
    #[pyo3(get, set)]
    pub locale: Option<String>,
    #[pyo3(get, set)]
    pub app_version: Option<String>,
    #[pyo3(get, set)]
    pub user_agent: Option<String>,
    #[pyo3(get)]
    pub custom: Option<PyObject>,
    #[pyo3(get)]
    pub custom_ids: Option<PyObject>,
    #[pyo3(get)]
    pub private_attributes: Option<PyObject>,
}

#[pymethods]
impl StatsigUserPy {
    #[new]
    #[pyo3(signature = (user_id, email=None, ip=None, country=None, locale=None, app_version=None, user_agent=None, custom=None, custom_ids=None, private_attributes=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: &str,
        email: Option<String>,
        ip: Option<String>,
        country: Option<String>,
        locale: Option<String>,
        app_version: Option<String>,
        user_agent: Option<String>,
        custom: Option<PyObject>,
        custom_ids: Option<PyObject>,
        private_attributes: Option<PyObject>,
        py: Python,
    ) -> PyResult<Self> {
        let mut user = StatsigUser::with_user_id(user_id.to_string());

        if let Some(e) = email.clone() {
            user.email = Some(DynamicValue::from(e));
        }
        if let Some(i) = ip.clone() {
            user.ip = Some(DynamicValue::from(i));
        }
        if let Some(c) = country.clone() {
            user.country = Some(DynamicValue::from(c));
        }
        if let Some(l) = locale.clone() {
            user.locale = Some(DynamicValue::from(l));
        }
        if let Some(a) = app_version.clone() {
            user.app_version = Some(DynamicValue::from(a));
        }
        if let Some(u) = user_agent.clone() {
            user.user_agent = Some(DynamicValue::from(u));
        }

        let custom_map = py_object_to_map(py, custom.as_ref())?;
        let custom_ids_map = py_object_to_map(py, custom_ids.as_ref())?;
        let private_attributes_map = py_object_to_map(py, private_attributes.as_ref())?;

        user.custom = custom_map;
        user.custom_ids = custom_ids_map;
        user.private_attributes = private_attributes_map;

        Ok(Self {
            inner: user,
            user_id: user_id.to_string(),
            email,
            ip,
            country,
            locale,
            app_version,
            user_agent,
            custom,
            custom_ids,
            private_attributes,
        })
    }
}
