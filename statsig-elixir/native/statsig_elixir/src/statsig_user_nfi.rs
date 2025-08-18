use crate::statsig_types_nfi::AllowedPrimitive;
use rustler::NifStruct;
use statsig_rust::{DynamicValue, StatsigUser as StatsigUserActual, StatsigUserBuilder};
use std::collections::HashMap;

macro_rules! to_value_with_dynamic {
    ($map:expr) => {{
        $map.into_iter()
            .map(|(key, value)| {
                let converted_value = match value {
                    Some(v) => v.into(),
                    None => DynamicValue::new(),
                };
                (key, converted_value)
            })
            .collect::<HashMap<String, DynamicValue>>()
    }};
}

#[derive(NifStruct)]
#[module = "Statsig.User"]
pub struct StatsigUser {
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub custom: Option<HashMap<String, Option<AllowedPrimitive>>>,
    pub custom_ids: Option<HashMap<String, String>>,
    pub private_attributes: Option<HashMap<String, Option<AllowedPrimitive>>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub locale: Option<String>,
    pub app_version: Option<String>,
}

impl From<StatsigUser> for StatsigUserActual {
    fn from(user: StatsigUser) -> Self {
        // We enforce either user id or custom ids being set on elixir side, so making assumption if no user id there must be custom_id
        match user.user_id {
            Some(id) => StatsigUserBuilder::new_with_user_id(id)
                .custom_ids(user.custom_ids)
                .app_version(user.app_version)
                .email(user.email)
                .ip(user.ip)
                .user_agent(user.user_agent)
                .locale(user.locale)
                .country(user.country)
                .custom(user.custom.map(|m| to_value_with_dynamic!(m)))
                .private_attributes(user.private_attributes.map(|m| to_value_with_dynamic!(m)))
                .build(),
            None => StatsigUserBuilder::new_with_custom_ids(user.custom_ids.unwrap_or_default())
                .app_version(user.app_version)
                .email(user.email)
                .ip(user.ip)
                .user_agent(user.user_agent)
                .locale(user.locale)
                .country(user.country)
                .custom(user.custom.map(|m| to_value_with_dynamic!(m)))
                .private_attributes(user.private_attributes.map(|m| to_value_with_dynamic!(m)))
                .build(),
        }
    }
}
