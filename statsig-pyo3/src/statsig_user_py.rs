use std::{str, sync::Arc};

use crate::pyo_utils::py_dict_to_map;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_w, user::user_data::UserData, DynamicValue, StatsigUser};

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
        let mut user_data = UserData {
            user_id: Some(DynamicValue::from(internal_user_id)),
            ..UserData::default()
        };

        if let Some(e) = email.clone() {
            user_data.email = Some(DynamicValue::from(e));
        }
        if let Some(i) = ip.clone() {
            user_data.ip = Some(DynamicValue::from(i));
        }
        if let Some(c) = country.clone() {
            user_data.country = Some(DynamicValue::from(c));
        }
        if let Some(l) = locale.clone() {
            user_data.locale = Some(DynamicValue::from(l));
        }
        if let Some(a) = app_version.clone() {
            user_data.app_version = Some(DynamicValue::from(a));
        }
        if let Some(u) = user_agent.clone() {
            user_data.user_agent = Some(DynamicValue::from(u));
        }

        let custom_map = custom.as_ref().map(|dict| py_dict_to_map(dict.bind(py)));
        let custom_ids_map = custom_ids
            .as_ref()
            .map(|dict| py_dict_to_map(dict.bind(py)));
        let private_attributes_map = private_attributes
            .as_ref()
            .map(|dict| py_dict_to_map(dict.bind(py)));

        user_data.custom = custom_map;
        user_data.custom_ids = custom_ids_map;
        user_data.private_attributes = private_attributes_map;

        Ok(Self {
            inner: StatsigUser {
                data: Arc::new(user_data),
            },
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
                    self.inner.set_user_id(user_id);
                }
            }
            "email" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.email = v.clone();
                    self.inner.set_email(v);
                }
            }
            "ip" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.ip = v.clone();
                    self.inner.set_ip(v);
                }
            }
            "country" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.country = v.clone();
                    self.inner.set_country(v);
                }
            }
            "locale" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.locale = v.clone();
                    self.inner.set_locale(v);
                }
            }
            "app_version" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.app_version = v.clone();
                    self.inner.set_app_version(v);
                }
            }
            "user_agent" => {
                if let Ok(v) = value.extract::<Option<String>>(py) {
                    self.user_agent = v.clone();
                    self.inner.set_user_agent(v);
                }
            }
            "custom_ids" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    let mut_data = Arc::make_mut(&mut self.inner.data);
                    mut_data.custom_ids = dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.custom_ids = dict;
                }
            }
            "custom" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    let mut_data = Arc::make_mut(&mut self.inner.data);
                    mut_data.custom = dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.custom = dict;
                }
            }
            "private_attributes" => {
                if let Ok(dict) = value.extract::<Option<Py<PyDict>>>(py) {
                    let mut_data = Arc::make_mut(&mut self.inner.data);
                    mut_data.private_attributes = dict.as_ref().map(|d| py_dict_to_map(d.bind(py)));
                    self.private_attributes = dict;
                }
            }
            _ => (), // Ignore other attributes
        }
        Ok(())
    }
}
