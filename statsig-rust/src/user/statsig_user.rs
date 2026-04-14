use crate::evaluation::dynamic_value::DynamicValue;
use crate::statsig_metadata;
use crate::{dyn_value, evaluation::dynamic_string::DynamicString};
use indexmap::IndexMap;
use std::sync::Arc;

use super::{
    into_optional::IntoOptional,
    unit_id::UnitID,
    user_data::{IntoOptionalUserDataMap, UserData, UserDataMap},
};

#[derive(Clone)]
pub struct StatsigUser {
    pub data: Arc<UserData>,

    pub(crate) sdk_version: &'static str,
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
    pub fn with_custom_ids<K, U, I>(custom_ids: I) -> Self
    where
        I: IntoIterator<Item = (K, U)>,
        K: Into<String>,
        U: Into<UnitID>,
    {
        let custom_ids: UserDataMap = custom_ids
            .into_iter()
            .map(|(k, v)| (k.into(), v.into().into()))
            .collect();

        Self::new(UserData {
            custom_ids: Some(custom_ids),
            ..UserData::default()
        })
    }

    pub fn new(inner: UserData) -> Self {
        Self {
            data: Arc::new(inner),
            sdk_version: statsig_metadata::SDK_VERSION,
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
        pub fn $getter_name(&self) -> Option<&UserDataMap> {
            self.data.$field.as_ref()
        }

        pub fn $setter_name(&mut self, value: impl IntoOptionalUserDataMap) {
            let mut_data = Arc::make_mut(&mut self.data);
            let value = match value.into_optional_user_data_map() {
                Some(value) => value,
                None => {
                    mut_data.$field = None;
                    return;
                }
            };

            mut_data.$field = Some(value);
        }
    };
}

impl StatsigUser {
    // ---------------------------------------- [User ID]

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

    // ---------------------------------------- [Custom IDs]

    pub fn get_custom_ids(&self) -> Option<IndexMap<&str, &str>> {
        let mapped = self
            .data
            .custom_ids
            .as_ref()?
            .iter()
            .map(entry_to_key_value_refs)
            .collect();

        Some(mapped)
    }

    pub fn set_custom_ids<K, U, I>(&mut self, custom_ids: I)
    where
        I: IntoIterator<Item = (K, U)>,
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

    // ---------------------------------------- [Unit ID]

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

    // ---------------------------------------- [ Statsig Environment ]

    pub fn get_statsig_environment(&self) -> Option<IndexMap<&str, &str>> {
        let mapped = self
            .data
            .statsig_environment
            .as_ref()?
            .iter()
            .map(entry_to_key_value_refs)
            .collect();

        Some(mapped)
    }

    pub fn set_statsig_environment<K, U, I>(&mut self, statsig_environment: Option<I>)
    where
        I: IntoIterator<Item = (K, U)>,
        K: Into<String>,
        U: Into<String>,
    {
        let mut_data = Arc::make_mut(&mut self.data);

        let statsig_environment = match statsig_environment {
            Some(v) => v,
            None => {
                mut_data.statsig_environment = None;
                return;
            }
        };

        let statsig_environment: UserDataMap = statsig_environment
            .into_iter()
            .map(|(k, v)| (k.into(), v.into().into()))
            .collect();

        mut_data.statsig_environment = Some(statsig_environment);
    }

    // ---------------------------------------- [ String Fields ]

    string_field_accessor!(self, get_email, set_email, email);
    string_field_accessor!(self, get_ip, set_ip, ip);
    string_field_accessor!(self, get_user_agent, set_user_agent, user_agent);
    string_field_accessor!(self, get_country, set_country, country);
    string_field_accessor!(self, get_locale, set_locale, locale);
    string_field_accessor!(self, get_app_version, set_app_version, app_version);

    // ---------------------------------------- [ Map Fields ]

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
