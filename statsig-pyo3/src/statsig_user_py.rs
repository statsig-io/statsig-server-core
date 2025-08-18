use crate::{
    unit_id_py::UnitIdPy,
    valid_primitives_py::{ValidPrimitivesPy, ValidPrimitivesPyRef},
};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::*;
use statsig_rust::{log_e, log_w, DynamicValue, StatsigUser, StatsigUserData};
use std::{collections::HashMap, str, sync::Arc};

const TAG: &str = stringify!(StatsigUserPy);

#[gen_stub_pyclass]
#[pyclass(name = "StatsigUser", module = "statsig_python_core")]
pub struct StatsigUserPy {
    pub inner: StatsigUser,
}

#[gen_stub_pymethods]
#[pymethods]
impl StatsigUserPy {
    #[new]
    #[pyo3(signature = (
        user_id=None, email=None, ip=None, country=None, locale=None, app_version=None, user_agent=None, custom=None, custom_ids=None, private_attributes=None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: Option<String>,
        email: Option<String>,
        ip: Option<String>,
        country: Option<String>,
        locale: Option<String>,
        app_version: Option<String>,
        user_agent: Option<String>,
        custom: Option<HashMap<String, Option<ValidPrimitivesPy>>>,
        custom_ids: Option<HashMap<String, UnitIdPy>>,
        private_attributes: Option<HashMap<String, Option<ValidPrimitivesPy>>>,
    ) -> PyResult<Self> {
        if user_id.is_none() && custom_ids.is_none() {
            log_w!(TAG, "Either `user_id` or `custom_ids` must be provided.");
        }

        let internal_user_id = user_id.unwrap_or_default();
        let mut user = StatsigUser::with_user_id(internal_user_id);

        user.set_email(email);
        user.set_ip(ip);
        user.set_country(country);
        user.set_locale(locale);
        user.set_app_version(app_version);
        user.set_user_agent(user_agent);

        let mut instance = Self { inner: user };
        instance.set_custom_ids(custom_ids);
        instance.set_custom(custom);
        instance.set_private_attributes(private_attributes);

        Ok(instance)
    }

    // ---------------------------------------- [UserID]

    #[getter]
    fn get_user_id(&self) -> &str {
        self.inner.get_user_id().unwrap_or_default()
    }

    #[setter]
    fn set_user_id(&mut self, value: Option<String>) {
        self.inner.set_user_id(value.unwrap_or_default());
    }

    // ---------------------------------------- [Email]

    #[getter]
    fn get_email(&self) -> Option<&str> {
        self.inner.get_email()
    }

    #[setter]
    fn set_email(&mut self, value: Option<String>) {
        self.inner.set_email(value);
    }

    // ---------------------------------------- [IP]

    #[getter]
    fn get_ip(&self) -> Option<&str> {
        self.inner.get_ip()
    }

    #[setter]
    fn set_ip(&mut self, value: Option<String>) {
        self.inner.set_ip(value);
    }

    // ---------------------------------------- [Country]

    #[getter]
    fn get_country(&self) -> Option<&str> {
        self.inner.get_country()
    }

    #[setter]
    fn set_country(&mut self, value: Option<String>) {
        self.inner.set_country(value);
    }

    // ---------------------------------------- [Locale]

    #[getter]
    fn get_locale(&self) -> Option<&str> {
        self.inner.get_locale()
    }

    #[setter]
    fn set_locale(&mut self, value: Option<String>) {
        self.inner.set_locale(value);
    }

    // ---------------------------------------- [App Version]

    #[getter]
    fn get_app_version(&self) -> Option<&str> {
        self.inner.get_app_version()
    }

    #[setter]
    fn set_app_version(&mut self, value: Option<String>) {
        self.inner.set_app_version(value);
    }

    // ---------------------------------------- [User Agent]

    #[getter]
    fn get_user_agent(&self) -> Option<&str> {
        self.inner.get_user_agent()
    }

    #[setter]
    fn set_user_agent(&mut self, value: Option<String>) {
        self.inner.set_user_agent(value);
    }

    // ---------------------------------------- [Custom IDs]

    #[getter]
    fn get_custom_ids(&self) -> Option<HashMap<&str, &str>> {
        let value = self.inner.data.custom_ids.as_ref()?;

        let mapped = value
            .iter()
            .map(|(k, v)| match &v.string_value {
                Some(dv) => (k.as_str(), dv.value.as_str()),
                None => (k.as_str(), ""),
            })
            .collect();

        Some(mapped)
    }

    #[setter]
    fn set_custom_ids(&mut self, value: Option<HashMap<String, UnitIdPy>>) {
        let converted = match value {
            Some(v) => v.into_iter().map(|(k, v)| (k, v.into_unit_id())).collect(),
            None => HashMap::new(),
        };
        self.inner.set_custom_ids(converted);
    }

    // ---------------------------------------- [Custom]

    #[getter]
    fn get_custom(&self) -> Option<HashMap<&str, Option<ValidPrimitivesPyRef<'_>>>> {
        get_map_field_ref(&self.inner.data.custom)
    }

    #[setter]
    fn set_custom(&mut self, value: Option<HashMap<String, Option<ValidPrimitivesPy>>>) {
        let mut converted: Option<HashMap<String, DynamicValue>> = None;

        if let Some(v) = value {
            converted = Some(
                v.into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                Some(v) => v.into_dynamic_value(),
                                None => DynamicValue::new(),
                            },
                        )
                    })
                    .collect(),
            );
        }

        self.inner.set_custom(converted);
    }

    // ---------------------------------------- [Private Attributes]

    #[getter]
    fn get_private_attributes(&self) -> Option<HashMap<&str, Option<ValidPrimitivesPyRef<'_>>>> {
        get_map_field_ref(&self.inner.data.private_attributes)
    }

    #[setter]
    fn set_private_attributes(
        &mut self,
        value: Option<HashMap<String, Option<ValidPrimitivesPy>>>,
    ) {
        let mut converted: Option<HashMap<String, DynamicValue>> = None;

        if let Some(v) = value {
            converted = Some(
                v.into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                Some(v) => v.into_dynamic_value(),
                                None => DynamicValue::new(),
                            },
                        )
                    })
                    .collect(),
            );
        }

        self.inner.set_private_attributes(converted);
    }

    // ---------------------------------------- [Pickling]

    pub fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = match self.inner.data.to_bytes() {
            Some(bytes) => bytes,
            None => {
                log_e!(TAG, "Failed to serialize StatsigUser.");
                vec![]
            }
        };

        Ok(PyBytes::new(py, &bytes))
    }

    pub fn __setstate__(&mut self, state: Bound<'_, PyBytes>) -> PyResult<()> {
        let bytes = state.as_bytes();
        let user_data = match serde_json::from_slice::<StatsigUserData>(bytes) {
            Ok(user_data) => user_data,
            Err(e) => {
                log_e!(TAG, "Failed to deserialize StatsigUser: {}", e);
                StatsigUserData::default()
            }
        };

        let inner = StatsigUser {
            data: Arc::new(user_data),
        };

        *self = StatsigUserPy { inner };
        Ok(())
    }
}

fn get_map_field_ref<'a>(
    field: &'a Option<HashMap<String, DynamicValue>>,
) -> Option<HashMap<&'a str, Option<ValidPrimitivesPyRef<'a>>>> {
    let value = field.as_ref()?;

    let mapped: HashMap<&'a str, Option<ValidPrimitivesPyRef<'a>>> = value
        .iter()
        .map(|(k, v)| {
            let value = ValidPrimitivesPyRef::from_dynamic_value(v);
            (k.as_str(), value)
        })
        .collect();

    Some(mapped)
}
