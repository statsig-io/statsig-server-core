use std::str;

use crate::pyo_utils::py_dict_to_map;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_w, DynamicValue, StatsigUser};

const TAG: &str = stringify!(StatsigUserPy);

#[gen_stub_pyclass]
#[pyclass(name = "StatsigUser")]
pub struct StatsigUserPy {
    pub inner: StatsigUser,

    #[pyo3(get, set)]
    pub user_id: Option<String>,
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
    #[pyo3(get, set)]
    pub custom: Option<Py<PyDict>>,
    #[pyo3(get, set)]
    pub custom_ids: Option<Py<PyDict>>,
    #[pyo3(get, set)]
    pub private_attributes: Option<Py<PyDict>>,
}

#[gen_stub_pymethods]
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
            log_w!(TAG, "Either `user_id` or `custom_ids` must be provided.");
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

    fn __setattr__(&mut self, name: &str, value: PyObject, py: Python) -> PyResult<()> {
        match name {
            "user_id" => {
                if let Ok(Some(user_id)) = value.extract::<Option<String>>(py) {
                    self.user_id = Some(user_id.clone());
                    self.inner.user_id = Some(DynamicValue::from(user_id.clone()));
                }
            }
            "email" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.email = v.clone();
                    self.inner.email = v.map(DynamicValue::from);
                }
            }
            "ip" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.ip = v.clone();
                    self.inner.ip = v.map(DynamicValue::from);
                }
            }
            "country" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.country = v.clone();
                    self.inner.country = v.map(DynamicValue::from);
                }
            }
            "locale" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.locale = v.clone();
                    self.inner.locale = v.map(DynamicValue::from);
                }
            }
            "app_version" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.app_version = v.clone();
                    self.inner.app_version = v.map(DynamicValue::from);
                }
            }
            "user_agent" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.user_agent = v.clone();
                    self.inner.user_agent = v.map(DynamicValue::from);
                }
            }
            "custom_ids" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    self.inner.custom_ids = dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.custom_ids = dict;
                }
            }
            "custom" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    self.inner.custom = dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.custom = dict;
                }
            }
            "private_attributes" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    self.inner.private_attributes =
                        dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.private_attributes = dict;
                }
            }
            _ => (), // Ignore other attributes
        }
        Ok(())
    }
}
