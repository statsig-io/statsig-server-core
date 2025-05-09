use super::unit_id::UnitID;
use super::StatsigUser;
use crate::dyn_value;
use crate::evaluation::dynamic_value::DynamicValue;
use std::collections::HashMap;

pub struct StatsigUserBuilder {
    pub user_id: Option<UnitID>,
    pub custom_ids: Option<HashMap<String, UnitID>>,

    pub email: Option<DynamicValue>,
    pub ip: Option<DynamicValue>,
    pub user_agent: Option<DynamicValue>,
    pub country: Option<DynamicValue>,
    pub locale: Option<DynamicValue>,
    pub app_version: Option<DynamicValue>,

    pub custom: Option<HashMap<String, DynamicValue>>,
    pub private_attributes: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUserBuilder {
    #[must_use]
    pub fn new_with_user_id(user_id: impl Into<UnitID>) -> Self {
        Self {
            user_id: Some(user_id.into()),
            ..Self::new()
        }
    }

    #[must_use]
    pub fn new_with_custom_ids<K, U>(custom_ids: HashMap<K, U>) -> Self
    where
        K: Into<String>,
        U: Into<UnitID>,
    {
        Self::new().custom_ids(Some(custom_ids))
    }

    fn new() -> Self {
        Self {
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

    pub fn user_id(mut self, user_id: Option<impl Into<UnitID>>) -> Self {
        if let Some(user_id) = user_id {
            self.user_id = Some(user_id.into());
        }
        self
    }

    pub fn custom_ids(
        mut self,
        custom_ids: Option<HashMap<impl Into<String>, impl Into<UnitID>>>,
    ) -> Self {
        if let Some(custom_ids) = custom_ids {
            self.custom_ids = Some(
                custom_ids
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            );
        }
        self
    }

    pub fn email(mut self, email: Option<String>) -> Self {
        if let Some(email) = email {
            self.email = Some(dyn_value!(email));
        }
        self
    }

    pub fn ip(mut self, ip: Option<String>) -> Self {
        if let Some(ip) = ip {
            self.ip = Some(dyn_value!(ip));
        }
        self
    }

    pub fn user_agent(mut self, user_agent: Option<String>) -> Self {
        if let Some(user_agent) = user_agent {
            self.user_agent = Some(dyn_value!(user_agent));
        }
        self
    }

    pub fn country(mut self, country: Option<String>) -> Self {
        if let Some(country) = country {
            self.country = Some(dyn_value!(country));
        }
        self
    }

    pub fn locale(mut self, locale: Option<String>) -> Self {
        if let Some(locale) = locale {
            self.locale = Some(dyn_value!(locale));
        }
        self
    }

    pub fn app_version(mut self, app_version: Option<String>) -> Self {
        if let Some(app_version) = app_version {
            self.app_version = Some(dyn_value!(app_version));
        }
        self
    }

    // todo: support HashMap<String, String | Number | Boolean | Array<String>>
    pub fn custom_from_str_map(mut self, custom: Option<HashMap<String, String>>) -> Self {
        if let Some(custom) = custom {
            self.custom = Some(convert_str_map_to_dyn_values(custom));
        }
        self
    }

    pub fn custom(mut self, custom: Option<HashMap<String, DynamicValue>>) -> Self {
        if let Some(custom) = custom {
            self.custom = Some(custom);
        }
        self
    }

    // todo: support HashMap<String, String | Number | Boolean | Array<String>>
    pub fn private_attributes_from_str_map(
        mut self,
        private_attributes: Option<HashMap<String, String>>,
    ) -> Self {
        if let Some(private_attributes) = private_attributes {
            self.private_attributes = Some(convert_str_map_to_dyn_values(private_attributes));
        }
        self
    }

    pub fn private_attributes(
        mut self,
        private_attributes: Option<HashMap<String, DynamicValue>>,
    ) -> Self {
        if let Some(private_attributes) = private_attributes {
            self.private_attributes = Some(private_attributes);
        }
        self
    }

    pub fn build(self) -> StatsigUser {
        StatsigUser {
            user_id: self.user_id.map(|u| u.into()),
            email: self.email,
            ip: self.ip,
            user_agent: self.user_agent,
            country: self.country,
            locale: self.locale,
            app_version: self.app_version,
            custom: self.custom,
            private_attributes: self.private_attributes,
            custom_ids: self
                .custom_ids
                .map(|m| m.into_iter().map(|(k, v)| (k, v.into())).collect()),
        }
    }
}

fn convert_str_map_to_dyn_values(
    custom_ids: HashMap<String, String>,
) -> HashMap<String, DynamicValue> {
    custom_ids
        .into_iter()
        .map(|(k, v)| (k, dyn_value!(v)))
        .collect()
}
