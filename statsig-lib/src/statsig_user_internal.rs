use crate::evaluation::dynamic_string::DynamicString;
use crate::evaluation::dynamic_value::DynamicValue;
use crate::StatsigUser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Serialize)]
pub struct StatsigUserLoggable {
    #[serde(flatten)]
    pub value: Value,
}

impl StatsigUserLoggable {
    pub fn new(user_internal: StatsigUserInternal) -> Self {
        let mut mut_user = user_internal;
        mut_user.user_data.private_attributes = None;

        Self {
            value: json!(mut_user),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatsigUserInternal {
    #[serde(flatten)]
    pub user_data: StatsigUser,

    pub statsig_environment: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUserInternal {
    pub fn new(user: &StatsigUser, environment: &Option<HashMap<String, DynamicValue>>) -> Self {
        Self {
            user_data: user.clone(),
            statsig_environment: environment.clone(),
        }
    }
    pub fn get_unit_id(&self, id_type: &DynamicString) -> Option<&DynamicValue> {
        if id_type.lowercased_value.eq("userid") {
            return self.user_data.user_id.as_ref();
        }

        let custom_ids = self.user_data.custom_ids.as_ref()?;

        if let Some(custom_id) = custom_ids.get(&id_type.value) {
            return Some(custom_id);
        }

        custom_ids.get(&id_type.lowercased_value)
    }

    pub fn get_user_value(&self, field: &Option<DynamicString>) -> Option<&DynamicValue> {
        let field = field.as_ref()?;

        let lowered_field = &field.lowercased_value;

        let str_value = match lowered_field as &str {
            "userid" | "user_id" => &self.user_data.user_id,
            "email" => &self.user_data.email,
            "ip" => &self.user_data.ip,
            "useragent" | "user_agent" => &self.user_data.user_agent,
            "country" => &self.user_data.country,
            "locale" => &self.user_data.locale,
            "appversion" | "app_version" => &self.user_data.app_version,
            _ => &None,
        };

        if str_value.is_some() {
            return str_value.as_ref();
        }

        if let Some(custom) = &self.user_data.custom {
            if let Some(found) = custom.get(&field.value) {
                return Some(found);
            }
            if let Some(lowered_found) = custom.get(lowered_field) {
                return Some(lowered_found);
            }
        }

        if let Some(private_attributes) = &self.user_data.private_attributes {
            if let Some(found) = private_attributes.get(&field.value) {
                return Some(found);
            }
            if let Some(lowered_found) = private_attributes.get(lowered_field) {
                return Some(lowered_found);
            }
        }

        None
    }

    pub fn get_value_from_environment(
        &self,
        field: &Option<DynamicString>,
    ) -> Option<&DynamicValue> {
        let field = field.as_ref()?;
        let env = self.statsig_environment.as_ref()?;

        if let Some(custom_id) = env.get(&field.value) {
            return Some(custom_id);
        }

        env.get(&field.lowercased_value)
    }
}
