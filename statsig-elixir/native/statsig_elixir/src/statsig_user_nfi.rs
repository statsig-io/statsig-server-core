use crate::statsig_types_nfi::AllowedPrimitive;
use rustler::NifStruct;
use statsig_rust::{
    statsig_user::StatsigUserBuilder, DynamicValue, StatsigUser as StatsigUserActual,
};
use std::collections::HashMap;

macro_rules! to_value_with_dynamic {
    ($map:expr) => {{
        $map.into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect::<HashMap<String, DynamicValue>>()
    }};
}

#[derive(NifStruct)]
#[module = "StatsigUser"]
pub struct StatsigUser {
    pub user_id: String,
    pub email: Option<String>,
    pub custom: Option<HashMap<String, AllowedPrimitive>>,
    pub custom_ids: Option<HashMap<String, String>>,
    pub private_attributes: Option<HashMap<String, AllowedPrimitive>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub locale: Option<String>,
    pub app_version: Option<String>,
}

impl From<StatsigUser> for StatsigUserActual {
    fn from(user: StatsigUser) -> Self {
        StatsigUserBuilder::new_with_user_id(user.user_id)
            .custom_ids(user.custom_ids)
            .app_version(user.app_version)
            .email(user.email)
            .ip(user.ip)
            .user_agent(user.user_agent)
            .locale(user.locale)
            .country(user.country)
            .custom(user.custom.map(|m| to_value_with_dynamic!(m)))
            .private_attributes(user.private_attributes.map(|m| to_value_with_dynamic!(m)))
            .build()
    }
}
