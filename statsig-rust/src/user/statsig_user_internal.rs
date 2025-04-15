use std::collections::HashMap;

use crate::evaluation::dynamic_value::DynamicValue;
use crate::StatsigUser;
use crate::{evaluation::dynamic_string::DynamicString, Statsig};
use serde::ser::SerializeMap;
use serde::Serialize;
use serde_json::json;

macro_rules! append_string_value {
    ($values:expr, $user_data:expr, $field:ident) => {
        if let Some(field) = &$user_data.$field {
            if let Some(string_value) = &field.string_value {
                $values += string_value;
                $values += "|";
            }
        }
    };
}

macro_rules! append_sorted_string_values {
    ($values:expr, $map:expr) => {
        if let Some(map) = $map {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            for key in keys {
                if let Some(string_value) = &map[key].string_value {
                    $values += key;
                    $values += string_value;
                }
            }
        }
    };
}

#[derive(Clone)]
pub struct StatsigUserInternal<'statsig, 'user> {
    pub user_data: &'user StatsigUser,

    pub statsig_instance: Option<&'statsig Statsig>,
}

impl<'statsig, 'user> StatsigUserInternal<'statsig, 'user> {
    pub fn new(user: &'user StatsigUser, statsig_instance: Option<&'statsig Statsig>) -> Self {
        Self {
            user_data: user,
            statsig_instance,
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
            "userid" => &self.user_data.user_id,
            "email" => &self.user_data.email,
            "ip" => &self.user_data.ip,
            "country" => &self.user_data.country,
            "locale" => &self.user_data.locale,
            "appversion" => &self.user_data.app_version,
            "useragent" => &self.user_data.user_agent,
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

        if let Some(instance) = &self.statsig_instance {
            if let Some(val) = instance.get_value_from_global_custom_fields(&field.value) {
                return Some(val);
            }

            if let Some(val) = instance.get_value_from_global_custom_fields(&field.lowercased_value)
            {
                return Some(val);
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

        let str_value_alt = match lowered_field as &str {
            "user_id" => &self.user_data.user_id,
            "app_version" => &self.user_data.app_version,
            "user_agent" => &self.user_data.user_agent,
            _ => &None,
        };

        if str_value_alt.is_some() {
            return str_value_alt.as_ref();
        }

        None
    }

    pub fn get_value_from_environment(
        &self,
        field: &Option<DynamicString>,
    ) -> Option<DynamicValue> {
        let field = field.as_ref()?;

        if let Some(result) = self.statsig_instance?.get_from_statsig_env(&field.value) {
            return Some(result);
        }

        self.statsig_instance?
            .get_from_statsig_env(&field.lowercased_value)
    }

    pub fn get_sampling_key(&self) -> String {
        let user_data = &self.user_data;

        let mut user_key = format!(
            "u:{};",
            user_data
                .user_id
                .as_ref()
                .and_then(|id| id.string_value.as_deref())
                .unwrap_or("")
        );

        if let Some(custom_ids) = user_data.custom_ids.as_ref() {
            for (key, val) in custom_ids {
                if let Some(string_value) = &val.string_value {
                    user_key.push_str(&format!("{key}:{string_value};"));
                }
            }
        };

        user_key
    }

    pub fn get_full_user_key(&self) -> String {
        let mut values = String::new();

        append_string_value!(values, self.user_data, app_version);
        append_string_value!(values, self.user_data, country);
        append_string_value!(values, self.user_data, email);
        append_string_value!(values, self.user_data, ip);
        append_string_value!(values, self.user_data, locale);
        append_string_value!(values, self.user_data, user_agent);
        append_string_value!(values, self.user_data, user_id);

        append_sorted_string_values!(values, &self.user_data.custom_ids);
        append_sorted_string_values!(values, &self.user_data.custom);
        append_sorted_string_values!(values, &self.user_data.private_attributes);

        if let Some(statsig_instance) = self.statsig_instance {
            statsig_instance.use_statsig_env(|env| {
                append_sorted_string_values!(values, env);
            });
        }

        values
    }
}

impl Serialize for StatsigUserInternal<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let inner_json = serde_json::to_value(self.user_data).map_err(serde::ser::Error::custom)?;

        let mut len = 1;
        if let serde_json::Value::Object(obj) = &inner_json {
            len += obj.len();
        }

        let mut state = serializer.serialize_map(Some(len))?;

        if let serde_json::Value::Object(obj) = &inner_json {
            for (k, v) in obj {
                state.serialize_entry(k, v)?;
            }
        }

        if let Some(statsig_instance) = self.statsig_instance {
            statsig_instance.use_global_custom_fields(|global_fields| {
                if is_none_or_empty(&self.user_data.custom.as_ref())
                    && is_none_or_empty(&global_fields)
                {
                    return Ok(());
                }

                let mut merged = HashMap::new();
                if let Some(user_custom) = &self.user_data.custom {
                    merged.extend(user_custom.iter());
                }

                if let Some(global) = global_fields {
                    merged.extend(global.iter());
                }

                state.serialize_entry("custom", &json!(merged))?;

                Ok(())
            })?;

            statsig_instance.use_statsig_env(|env| {
                if let Some(env) = env {
                    state.serialize_entry("statsigEnvironment", &json!(env))?;
                }

                Ok(())
            })?;
        }

        state.end()
    }
}

fn is_none_or_empty(opt_vec: &Option<&HashMap<String, DynamicValue>>) -> bool {
    match opt_vec {
        None => true,
        Some(vec) => vec.is_empty(),
    }
}
