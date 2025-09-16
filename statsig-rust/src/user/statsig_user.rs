use crate::evaluation::dynamic_value::DynamicValue;
use crate::{dyn_value, evaluation::dynamic_string::DynamicString};
use std::{collections::HashMap, sync::Arc};

use super::{into_optional::IntoOptional, unit_id::UnitID, user_data::UserData};

#[derive(Clone)]
pub struct StatsigUser {
    pub data: Arc<UserData>,
}

impl StatsigUser {
    #[must_use]
    pub fn with_user_id(user_id: impl Into<UnitID>) -> Self {
        let unit_id: UnitID = user_id.into();
        Self::new(UserData {
            user_id: Some(unit_id.into()),
            ..UserData::default()
        })
    }

    #[must_use]
    pub fn with_custom_ids<K, U>(custom_ids: HashMap<K, U>) -> Self
    where
        K: Into<String>,
        U: Into<UnitID>,
    {
        let custom_ids: HashMap<String, DynamicValue> = custom_ids
            .into_iter()
            .map(|(k, v)| (k.into(), v.into().into()))
            .collect();

        Self::new(UserData {
            custom_ids: Some(custom_ids),
            ..UserData::default()
        })
    }

    pub(crate) fn new(inner: UserData) -> Self {
        Self {
            data: Arc::new(inner),
        }
    }
}

// -------------------------------------------------------------------------------- [Getters/Setters]

macro_rules! string_field_accessor {
    ($self:ident, $getter_name:ident, $setter_name:ident, $field:ident) => {
        pub fn $getter_name(&self) -> Option<&str> {
            self.data
                .$field
                .as_ref()?
                .string_value
                .as_ref()
                .map(|s| s.value.as_str())
        }

        pub fn $setter_name(&mut self, value: impl IntoOptional<String>) {
            let value = value.into_optional();
            let mut_data = Arc::make_mut(&mut self.data);
            match value {
                Some(value) => {
                    mut_data.$field = Some(dyn_value!(value));
                }
                None => mut_data.$field = None,
            }
        }
    };
}

macro_rules! map_field_accessor {
    ($self:ident, $getter_name:ident, $setter_name:ident, $field:ident) => {
        pub fn $getter_name(&self) -> Option<&HashMap<String, DynamicValue>> {
            self.data.$field.as_ref()
        }

        pub fn $setter_name<K, V>(&mut self, value: impl IntoOptional<HashMap<K, V>>)
        where
            K: Into<String>,
            V: Into<DynamicValue>,
        {
            let mut_data = Arc::make_mut(&mut self.data);
            let value = match value.into_optional() {
                Some(value) => value,
                None => {
                    mut_data.$field = None;
                    return;
                }
            };

            mut_data.$field = Some(
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
        self.data
            .user_id
            .as_ref()?
            .string_value
            .as_ref()
            .map(|s| s.value.as_str())
    }

    pub fn set_user_id(&mut self, user_id: impl Into<UnitID>) {
        let unit_id = user_id.into();
        let mut_data = Arc::make_mut(&mut self.data);
        mut_data.user_id = Some(unit_id.into());
    }

    pub fn get_custom_ids(&self) -> Option<HashMap<&str, &str>> {
        let mapped = self
            .data
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

        let mut_data = Arc::make_mut(&mut self.data);
        mut_data.custom_ids = Some(custom_ids);
    }

    pub fn get_unit_id(&self, id_type: &DynamicString) -> Option<&DynamicValue> {
        if id_type.lowercased_value.eq("userid") {
            return self.data.user_id.as_ref();
        }

        let custom_ids = self.data.custom_ids.as_ref()?;

        if let Some(custom_id) = custom_ids.get(id_type.value.as_str()) {
            return Some(custom_id);
        }

        custom_ids.get(id_type.lowercased_value.as_str())
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
