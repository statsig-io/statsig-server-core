use crate::dyn_value;
use crate::evaluation::dynamic_value::DynamicValue;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

use super::{into_optional::IntoOptional, unit_id::UnitID};

#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigUser {
    #[serde(rename = "userID")]
    pub user_id: Option<DynamicValue>,
    #[serde(rename = "customIDs")]
    pub custom_ids: Option<HashMap<String, DynamicValue>>,

    pub email: Option<DynamicValue>,
    pub ip: Option<DynamicValue>,
    pub user_agent: Option<DynamicValue>,
    pub country: Option<DynamicValue>,
    pub locale: Option<DynamicValue>,
    pub app_version: Option<DynamicValue>,

    pub custom: Option<HashMap<String, DynamicValue>>,

    #[serde(skip_serializing)]
    pub private_attributes: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUser {
    #[must_use]
    pub fn with_user_id(user_id: impl Into<UnitID>) -> Self {
        let unit_id = user_id.into();
        StatsigUser {
            user_id: Some(unit_id.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_custom_ids<K, U>(custom_ids: HashMap<K, U>) -> Self
    where
        K: Into<String>,
        U: Into<UnitID>,
    {
        let custom_ids = custom_ids
            .into_iter()
            .map(|(k, v)| (k.into(), v.into().into()))
            .collect();

        StatsigUser {
            custom_ids: Some(custom_ids),
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

// -------------------------------------------------------------------------------- [Getters/Setters]

macro_rules! string_field_accessor {
    ($self:ident, $getter_name:ident, $setter_name:ident, $field:ident) => {
        pub fn $getter_name(&self) -> Option<&str> {
            self.$field
                .as_ref()?
                .string_value
                .as_ref()
                .map(|s| s.value.as_str())
        }

        pub fn $setter_name(&mut self, value: impl IntoOptional<String>) {
            let value = value.into_optional();
            match value {
                Some(value) => {
                    self.$field = Some(dyn_value!(value));
                }
                None => self.$field = None,
            }
        }
    };
}

macro_rules! map_field_accessor {
    ($self:ident, $getter_name:ident, $setter_name:ident, $field:ident) => {
        pub fn $getter_name(&self) -> Option<&HashMap<String, DynamicValue>> {
            self.$field.as_ref()
        }

        pub fn $setter_name<K, V>(&mut self, value: impl IntoOptional<HashMap<K, V>>)
        where
            K: Into<String>,
            V: Into<DynamicValue>,
        {
            let value = match value.into_optional() {
                Some(value) => value,
                None => {
                    self.$field = None;
                    return;
                }
            };

            self.$field = Some(
                value
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            );
        }
    };
}

impl StatsigUser {
    pub fn get_user_id(&self) -> Option<&str> {
        self.user_id
            .as_ref()?
            .string_value
            .as_ref()
            .map(|s| s.value.as_str())
    }

    pub fn set_user_id(&mut self, user_id: impl Into<UnitID>) {
        let unit_id = user_id.into();
        self.user_id = Some(unit_id.into());
    }

    pub fn get_custom_ids(&self) -> Option<HashMap<&str, &str>> {
        let mapped = self
            .custom_ids
            .as_ref()?
            .iter()
            .map(entry_to_key_value_refs)
            .collect();

        Some(mapped)
    }

    pub fn set_custom_ids<K, U>(&mut self, custom_ids: HashMap<K, U>)
    where
        K: Into<String>,
        U: Into<UnitID>,
    {
        let custom_ids = custom_ids
            .into_iter()
            .map(|(k, v)| (k.into(), v.into().into()))
            .collect();

        self.custom_ids = Some(custom_ids);
    }

    string_field_accessor!(self, get_email, set_email, email);
    string_field_accessor!(self, get_ip, set_ip, ip);
    string_field_accessor!(self, get_user_agent, set_user_agent, user_agent);
    string_field_accessor!(self, get_country, set_country, country);
    string_field_accessor!(self, get_locale, set_locale, locale);
    string_field_accessor!(self, get_app_version, set_app_version, app_version);

    map_field_accessor!(self, get_custom, set_custom, custom);
    map_field_accessor!(
        self,
        get_private_attributes,
        set_private_attributes,
        private_attributes
    );
}

// -------------------------------------------------------------------------------- [Helpers]

fn entry_to_key_value_refs<'a>(entry: (&'a String, &'a DynamicValue)) -> (&'a str, &'a str) {
    let (key, value) = entry;

    (
        key.as_str(),
        value
            .string_value
            .as_ref()
            .map(|s| s.value.as_str())
            .unwrap_or(""),
    )
}
