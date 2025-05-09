use crate::dyn_value;
use crate::evaluation::dynamic_value::DynamicValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigUser {
    #[serde(rename = "userID", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<DynamicValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, DynamicValue>>,

    #[serde(skip_serializing)]
    pub private_attributes: Option<HashMap<String, DynamicValue>>,

    #[serde(rename = "customIDs", skip_serializing_if = "Option::is_none")]
    pub custom_ids: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUser {
    #[must_use]
    pub fn with_user_id(user_id: String) -> Self {
        StatsigUser {
            user_id: Some(dyn_value!(user_id)),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_custom_ids<V>(custom_ids: HashMap<String, V>) -> Self
    where
        V: Into<DynamicValue>,
    {
        StatsigUser {
            custom_ids: Some(custom_ids.into_iter().map(|(k, v)| (k, v.into())).collect()),
            ..Self::default()
        }
    }

    fn default() -> Self {
        StatsigUser {
            user_id: None,
            email: None,
            ip: None,
            user_agent: None,
            country: None,
            locale: None,
            app_version: None,
            custom: None,
            private_attributes: None,
            custom_ids: None,
        }
    }
}
