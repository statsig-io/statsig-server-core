use crate::pyo_utils::py_dict_to_map;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sigstat::{DynamicValue, StatsigUser};

#[pyclass(name = "StatsigUser")]
pub struct StatsigUserPy {
    pub inner: StatsigUser,

    #[pyo3(get)]
    pub user_id: Option<String>,
    #[pyo3(get)]
    pub email: Option<String>,
    #[pyo3(get)]
    pub ip: Option<String>,
    #[pyo3(get)]
    pub country: Option<String>,
    #[pyo3(get)]
    pub locale: Option<String>,
    #[pyo3(get)]
    pub app_version: Option<String>,
    #[pyo3(get)]
    pub user_agent: Option<String>,
    #[pyo3(get)]
    pub custom: Option<Py<PyDict>>,
    #[pyo3(get)]
    pub custom_ids: Option<Py<PyDict>>,
    #[pyo3(get)]
    pub private_attributes: Option<Py<PyDict>>,
}

#[pymethods]
impl StatsigUserPy {
    #[new]
    #[pyo3(signature = (user_id=None, email=None, ip=None, country=None, locale=None, app_version=None, user_agent=None, custom=None, custom_ids=None, private_attributes=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: Option<String>,
        email: Option<String>,
        ip: Option<String>,
        country: Option<String>,
        locale: Option<String>,
        app_version: Option<String>,
        user_agent: Option<String>,
        custom: Option<Py<PyDict>>,
        custom_ids: Option<Py<PyDict>>,
        private_attributes: Option<Py<PyDict>>,
        py: Python,
    ) -> PyResult<Self> {
        if user_id.is_none() && custom_ids.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "[StatsigUser] Either `user_id` or `custom_ids` must be provided.",
            ));
        }

        let internal_user_id = user_id.clone().unwrap_or_default();
        let mut user = StatsigUser::with_user_id(internal_user_id);

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

        let custom_map = custom.as_ref().map(|dict| py_dict_to_map(dict.bind(py)));
        let custom_ids_map = custom_ids
            .as_ref()
            .map(|dict| py_dict_to_map(dict.bind(py)));
        let private_attributes_map = private_attributes
            .as_ref()
            .map(|dict| py_dict_to_map(dict.bind(py)));

        user.custom = custom_map;
        user.custom_ids = custom_ids_map;
        user.private_attributes = private_attributes_map;

        Ok(Self {
            inner: user,
            user_id,
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
