use std::collections::HashMap;

use crate::evaluation::dynamic_value::DynamicValue;
use crate::StatsigUser;
use crate::{evaluation::dynamic_string::DynamicString, Statsig};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Deserialize, Serialize)]
pub struct StatsigUserLoggable {
    #[serde(flatten)]
    pub value: Value,
}

impl StatsigUserLoggable {
    pub fn new(user_internal: &StatsigUserInternal) -> Self {
        Self {
            value: json!(user_internal),
        }
    }

    pub fn get_sampling_key(&self) -> String {
        let user_data = &self.value;
        let user_id = user_data
            .get("userID")
            .map(|x| x.as_str())
            .unwrap_or_default()
            .unwrap_or_default();

        // done this way for perf reasons
        let mut user_key = String::from("u:");
        user_key += user_id;
        user_key += ";";

        let custom_ids = user_data
            .get("customIDs")
            .map(|x| x.as_object())
            .unwrap_or_default();

        if let Some(custom_ids) = custom_ids {
            for (key, val) in custom_ids.iter() {
                if let Some(string_value) = &val.as_str() {
                    user_key += key;
                    user_key += ":";
                    user_key += string_value;
                    user_key += ";";
                }
            }
        };

        user_key
    }
}

#[derive(Clone)]
pub struct StatsigUserInternal<'statsig, 'user> {
    pub user_data: &'user StatsigUser,

    pub statsig_instance: Option<&'statsig Statsig>,
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

            statsig_instance
                .use_statsig_env(|env| state.serialize_entry("statsigEnvironment", &json!(env)))?;
        }

        state.end()
    }
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

    pub fn to_loggable(&self) -> StatsigUserLoggable {
        StatsigUserLoggable::new(self)
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
}

fn is_none_or_empty(opt_vec: &Option<&HashMap<String, DynamicValue>>) -> bool {
    match opt_vec {
        None => true,
        Some(vec) => vec.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use crate::StatsigOptions;

    use super::*;
    use std::{collections::HashMap, sync::Arc};

    fn create_test_user(custom_fields: Option<HashMap<String, DynamicValue>>) -> StatsigUser {
        StatsigUser {
            custom: custom_fields,
            ..StatsigUser::with_user_id("test_user_id".to_string())
        }
    }

    fn create_statsig_instance(
        global_custom: Option<HashMap<String, DynamicValue>>,
        environment: Option<String>,
    ) -> Statsig {
        let options = StatsigOptions {
            global_custom_fields: global_custom,
            environment,
            ..Default::default()
        };
        Statsig::new("secret-key", Some(Arc::new(options)))
    }

    fn serialize_and_deserialize(user_internal: &StatsigUserInternal) -> StatsigUserLoggable {
        let loggable = user_internal.to_loggable();
        let serialized = serde_json::to_string(&loggable).unwrap();
        serde_json::from_str(&serialized).unwrap()
    }

    #[test]
    fn test_loggable_strips_private_attributes() {
        let mut private_attrs = HashMap::new();
        private_attrs.insert("secret".to_string(), DynamicValue::from("sensitive_data"));

        let user = StatsigUser {
            private_attributes: Some(private_attrs),
            ..StatsigUser::with_user_id("test_user".to_string())
        };

        let user_internal = StatsigUserInternal::new(&user, None);
        let loggable = user_internal.to_loggable();

        let private_attrs = loggable.value.get("private_attributes");
        assert!(private_attrs.is_none());
    }

    #[test]
    fn test_serialization_with_global_custom_fields() {
        let user = create_test_user(None);
        let global_custom = HashMap::from([(
            "test_custom_field".to_string(),
            DynamicValue::from("test_custom_field_value"),
        )]);

        let statsig = create_statsig_instance(Some(global_custom), Some("dev".to_string()));
        let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
        let deserialized = serialize_and_deserialize(&user_internal);

        let deserialized_user_id = deserialized.value.get("userID").cloned();
        assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

        let deserialized_statsig_env = deserialized.value.get("statsigEnvironment").cloned();
        assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

        let deserialized_custom = deserialized.value.get("custom").cloned();
        assert_eq!(
            deserialized_custom,
            Some(json!({"test_custom_field": "test_custom_field_value"}))
        );
    }

    #[test]
    fn test_serialization_with_no_custom_fields() {
        let user = create_test_user(None);
        let user_internal = StatsigUserInternal::new(&user, None);
        let deserialized = serialize_and_deserialize(&user_internal);

        let deserialized_user_id = deserialized.value.get("userID").cloned();
        assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

        assert_eq!(deserialized.value.as_object().unwrap().keys().len(), 1);
    }

    #[test]
    fn test_serialization_with_local_custom_fields() {
        let custom_fields = HashMap::from([(
            "test_custom_field".to_string(),
            DynamicValue::from("test_custom_field_value"),
        )]);
        let user = create_test_user(Some(custom_fields));

        let user_internal = StatsigUserInternal::new(&user, None);
        let deserialized = serialize_and_deserialize(&user_internal);

        let deserialized_user_id = deserialized.value.get("userID").cloned();
        assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

        let deserialized_custom = deserialized.value.get("custom").cloned();
        assert_eq!(
            deserialized_custom,
            Some(json!({"test_custom_field": "test_custom_field_value"}))
        );

        assert_eq!(deserialized.value.as_object().unwrap().keys().len(), 2);
    }

    #[test]
    fn test_serialization_with_local_custom_fields_and_global_custom_fields() {
        let local_custom = HashMap::from([(
            "test_local_custom_field".to_string(),
            DynamicValue::from("test_local_custom_field_value"),
        )]);
        let user = create_test_user(Some(local_custom));

        let global_custom = HashMap::from([(
            "test_custom_field".to_string(),
            DynamicValue::from("test_custom_field_value"),
        )]);

        let statsig = create_statsig_instance(Some(global_custom), Some("dev".to_string()));
        let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
        let deserialized = serialize_and_deserialize(&user_internal);

        let deserialized_user_id = deserialized.value.get("userID").cloned();
        assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

        let deserialized_statsig_env = deserialized.value.get("statsigEnvironment").cloned();
        assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

        let deserialized_custom = deserialized.value.get("custom").cloned();
        assert_eq!(
            deserialized_custom,
            Some(json!({
                    "test_local_custom_field": "test_local_custom_field_value",
                    "test_custom_field": "test_custom_field_value"
            }))
        );
    }

    #[test]
    fn test_serialization_has_correct_keys() {
        let user = StatsigUser {
            custom: Some(HashMap::from([(
                "test_custom_field".to_string(),
                DynamicValue::from("test_custom_field_value"),
            )])),
            private_attributes: Some(HashMap::from([(
                "test_private_attribute".to_string(),
                DynamicValue::from("test_private_attribute_value"),
            )])),
            email: Some("test_email".into()),
            ip: Some("test_ip".into()),
            user_agent: Some("test_user_agent".into()),
            country: Some("test_country".into()),
            locale: Some("test_locale".into()),
            app_version: Some("test_app_version".into()),
            custom_ids: Some(HashMap::from([(
                "test_custom_id".to_string(),
                DynamicValue::from("test_custom_id_value"),
            )])),
            ..StatsigUser::with_user_id("test_user_id".to_string())
        };
        let user_internal = StatsigUserInternal::new(&user, None);
        let deserialized = serialize_and_deserialize(&user_internal);

        let keys = deserialized.value.as_object().unwrap();
        assert!(keys.contains_key("userID"));
        assert!(keys.contains_key("email"));
        assert!(keys.contains_key("ip"));
        assert!(keys.contains_key("userAgent"));
        assert!(keys.contains_key("country"));
        assert!(keys.contains_key("locale"));
        assert!(keys.contains_key("appVersion"));
        assert!(keys.contains_key("custom"));
        assert!(keys.contains_key("customIDs"));

        assert!(!keys.contains_key("privateAttributes"));
    }
}
