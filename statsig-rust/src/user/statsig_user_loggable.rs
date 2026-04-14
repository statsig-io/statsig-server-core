use super::user_data::{UserData, UserDataMap};
use crate::{DynamicValue, StatsigUser};
use serde::{
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Serialize,
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

const TAG: &str = "StatsigUserLoggable";

#[derive(Clone, Default)]
pub struct StatsigUserLoggable {
    pub data: Arc<UserData>,
    pub environment: Option<UserDataMap>,
    pub global_custom: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUserLoggable {
    pub fn new(
        user_inner: &Arc<UserData>,
        environment: Option<UserDataMap>,
        global_custom: Option<HashMap<String, DynamicValue>>,
    ) -> Self {
        Self {
            data: user_inner.clone(),
            environment,
            global_custom,
        }
    }

    pub fn null() -> Self {
        Self::default()
    }

    pub fn default_console_capture_user(
        environment: Option<UserDataMap>,
        global_custom: Option<HashMap<String, DynamicValue>>,
    ) -> Self {
        Self::new(
            &StatsigUser::with_user_id("console-capture-user").data,
            environment,
            global_custom,
        )
    }
}

// ----------------------------------------------------- [Serialization]

impl Serialize for StatsigUserLoggable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(TAG, 10)?;

        let data = self.data.as_ref();
        serialize_data_field(&mut state, "userID", &data.user_id)?;
        serialize_data_field(&mut state, "customIDs", &data.custom_ids)?;
        serialize_data_field(&mut state, "email", &data.email)?;
        serialize_data_field(&mut state, "ip", &data.ip)?;
        serialize_data_field(&mut state, "userAgent", &data.user_agent)?;
        serialize_data_field(&mut state, "country", &data.country)?;
        serialize_data_field(&mut state, "locale", &data.locale)?;
        serialize_data_field(&mut state, "appVersion", &data.app_version)?;

        serialize_custom_field(&mut state, &data.custom, &self.global_custom)?;
        serialize_data_field(&mut state, "statsigEnvironment", &self.environment)?;

        // DO NOT SERIALIZE "privateAttributes"

        state.end()
    }
}

impl<'de> Deserialize<'de> for StatsigUserLoggable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        let env = value["statsigEnvironment"].take();
        let data = serde_json::from_value::<UserData>(value).map_err(|e| {
            serde::de::Error::custom(format!("Error deserializing StatsigUserInner: {e}"))
        })?;

        let environment = serde_json::from_value::<Option<UserDataMap>>(env).map_err(|e| {
            serde::de::Error::custom(format!("Error deserializing StatsigUserInner: {e}"))
        })?;

        Ok(StatsigUserLoggable {
            data: Arc::new(data),
            environment,
            global_custom: None, // there is no way to discern between user-defined and global custom fields
        })
    }
}

fn serialize_data_field<S, T>(
    state: &mut S,
    field: &'static str,
    value: &Option<T>,
) -> Result<(), S::Error>
where
    S: SerializeStruct,
    T: Serialize,
{
    if let Some(value) = value {
        state.serialize_field(field, value)?;
    }
    Ok(())
}

fn serialize_custom_field<S>(
    state: &mut S,
    custom: &Option<UserDataMap>,
    global_custom: &Option<HashMap<String, DynamicValue>>,
) -> Result<(), S::Error>
where
    S: SerializeStruct,
{
    if global_custom.is_none() && custom.is_none() {
        return Ok(());
    }

    state.serialize_field(
        "custom",
        &MergedCustomFields {
            custom: custom.as_ref(),
            global_custom: global_custom.as_ref(),
        },
    )
}

struct MergedCustomFields<'a> {
    custom: Option<&'a UserDataMap>,
    global_custom: Option<&'a HashMap<String, DynamicValue>>,
}

impl MergedCustomFields<'_> {
    fn serialized_len(&self) -> usize {
        let global_len = self.global_custom.map_or(0, HashMap::len);
        let custom_only_len = self.custom.map_or(0, |custom| {
            custom
                .keys()
                .filter(|key| {
                    !self
                        .global_custom
                        .is_some_and(|global_custom| global_custom.contains_key(*key))
                })
                .count()
        });

        global_len + custom_only_len
    }
}

impl Serialize for MergedCustomFields<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.serialized_len()))?;

        if let Some(global_custom) = self.global_custom {
            for (key, global_value) in global_custom {
                let value = self
                    .custom
                    .and_then(|custom| custom.get(key))
                    .unwrap_or(global_value);
                map.serialize_entry(key, value)?;
            }
        }

        if let Some(custom) = self.custom {
            for (key, value) in custom {
                if self
                    .global_custom
                    .is_some_and(|global_custom| global_custom.contains_key(key))
                {
                    continue;
                }

                map.serialize_entry(key, value)?;
            }
        }

        map.end()
    }
}
