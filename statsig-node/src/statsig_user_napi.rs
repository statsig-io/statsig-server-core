use std::collections::HashMap;

use napi_derive::napi;
use serde_json::Value;
use sigstat::{log_w, DynamicValue, StatsigUser as StatsigUserActual};

const TAG: &str = "StatsigUserNapi";

#[napi]
pub struct StatsigUser {
    inner: StatsigUserActual,
}

#[napi]
impl StatsigUser {
    #[napi]
    pub fn with_user_id(user_id: String) -> Self {
        Self {
            inner: StatsigUserActual::with_user_id(user_id),
        }
    }

    #[napi]
    pub fn with_custom_ids(custom_ids: HashMap<String, Value>) -> Self {
        let mut converted: HashMap<String, String> = HashMap::new();

        for (key, value) in custom_ids {
            if let Some(v) = value.as_str() {
                converted.insert(key, v.to_string());
            } else {
                log_w!(TAG, "Custom ID value is not a string: {}", value);
            }
        }

        Self {
            inner: StatsigUserActual::with_custom_ids(converted),
        }
    }

    pub fn as_inner(&self) -> &StatsigUserActual {
        &self.inner
    }
}

macro_rules! add_hashmap_getter_setter {
    ($field_name:expr, $field_accessor:ident, $setter_name:ident) => {
        #[napi]
        impl StatsigUser {
            #[napi(getter, js_name = $field_name)]
            pub fn $field_accessor(&self) -> Option<HashMap<String, String>> {
                let mut result: HashMap<String, String> = HashMap::new();

                let value_map = match &self.inner.$field_accessor {
                    Some(value) => value,
                    _ => return None,
                };

                for (key, value) in value_map {
                    if let Some(value) = &value.string_value {
                        result.insert(key.to_string(), value.clone());
                    }
                }

                Some(result)
            }

            #[napi(setter, js_name = $field_name)]
            pub fn $setter_name(&mut self, value: Option<HashMap<String, Value>>) {
                let value = match value {
                    Some(value) => value,
                    _ => {
                        self.inner.$field_accessor = None;
                        return;
                    }
                };

                let mut converted: HashMap<String, DynamicValue> = HashMap::new();

                for (key, value) in value {
                    if $field_name == "customIDs" && !value.is_string() {
                        log_w!(TAG, "Custom ID value is not a string: {}", value);
                        continue;
                    }

                    converted.insert(key, DynamicValue::from(value));
                }

                self.inner.$field_accessor = Some(converted);
            }
        }
    };
}

macro_rules! add_string_getter_setter {
    ($field_name:expr, $field_accessor:ident, $setter_name:ident) => {
        #[napi]
        impl StatsigUser {
            #[napi(getter, js_name = $field_name)]
            pub fn $field_accessor(&self) -> Option<String> {
                match &self.inner.$field_accessor {
                    Some(value) => value.string_value.clone(),
                    _ => None,
                }
            }

            #[napi(setter, js_name = $field_name)]
            pub fn $setter_name(&mut self, value: Option<Value>) {
                match value {
                    Some(value) => self.inner.$field_accessor = Some(value.into()),
                    _ => self.inner.$field_accessor = None,
                }
            }
        }
    };
}

add_hashmap_getter_setter!("customIDs", custom_ids, set_custom_ids);
add_hashmap_getter_setter!("custom", custom, set_custom);
add_hashmap_getter_setter!(
    "privateAttributes",
    private_attributes,
    set_private_attributes
);

add_string_getter_setter!("userID", user_id, set_user_id);
add_string_getter_setter!("email", email, set_email);
add_string_getter_setter!("ip", ip, set_ip);
add_string_getter_setter!("userAgent", user_agent, set_user_agent);
add_string_getter_setter!("country", country, set_country);
add_string_getter_setter!("locale", locale, set_locale);
add_string_getter_setter!("appVersion", app_version, set_app_version);
