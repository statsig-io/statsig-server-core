use pyo3::prelude::*;
use sigstat::{DynamicValue, StatsigUser};
use std::collections::HashMap;

#[pyclass(name = "StatsigUser")]
pub struct StatsigUserPy {
    pub inner: StatsigUser,
}

#[pymethods]
impl StatsigUserPy {
    #[new]
    #[pyo3(signature = (user_id, user_json=None))]
    pub fn new(user_id: &str, user_json: Option<String>) -> Self {
        let mut user = StatsigUser::with_user_id(user_id.to_string());
        if user_json.is_none() {
            return Self { inner: user };
        }
        let parsed_fields: HashMap<String, DynamicValue> =
            serde_json::from_str(&user_json.unwrap()).unwrap();
        let email = parsed_fields.get("email");
        user.email = email.cloned();

        let ip = parsed_fields.get("ip");
        user.ip = ip.cloned();

        let country = parsed_fields.get("country");
        user.country = country.cloned();

        let locale = parsed_fields.get("locale");
        user.locale = locale.cloned();

        let app_version = parsed_fields.get("appVersion");
        user.app_version = app_version.cloned();

        let user_agent = parsed_fields.get("userAgent");
        user.user_agent = user_agent.cloned();

        if let Some(custom) = parsed_fields.get("custom") {
            user.custom = custom.object_value.clone();
        }
        if let Some(private_attributes) = parsed_fields.get("privateAttributes") {
            user.private_attributes = private_attributes.object_value.clone();
        }
        if let Some(custom_ids) = parsed_fields.get("customIDs") {
            user.custom_ids = custom_ids.object_value.clone();
        }
        Self { inner: user }
    }
}
