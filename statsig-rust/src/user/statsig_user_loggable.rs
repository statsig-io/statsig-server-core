use super::user_data::UserData;
use crate::DynamicValue;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

const TAG: &str = "StatsigUserLoggable";

#[derive(Clone, Default)]
pub struct StatsigUserLoggable {
    pub data: Arc<UserData>,
    pub environment: Option<HashMap<String, DynamicValue>>,
    pub global_custom: Option<HashMap<String, DynamicValue>>,
}

impl StatsigUserLoggable {
    pub fn new(
        user_inner: &Arc<UserData>,
        environment: Option<HashMap<String, DynamicValue>>,
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

        let environment = serde_json::from_value::<Option<HashMap<String, DynamicValue>>>(env)
            .map_err(|e| {
                serde::de::Error::custom(format!("Error deserializing StatsigUserInner: {e}"))
            })?;

        Ok(StatsigUserLoggable {
            data: Arc::new(data),
            environment,
            global_custom: None,
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
    custom: &Option<HashMap<String, DynamicValue>>,
    global_custom: &Option<HashMap<String, DynamicValue>>,
) -> Result<(), S::Error>
where
    S: SerializeStruct,
{
    if global_custom.is_none() && custom.is_none() {
        return Ok(());
    }

    let mut map = HashMap::new();

    if let Some(global_custom) = global_custom.as_ref() {
        for (k, v) in global_custom {
            map.insert(k, v);
        }
    }

    if let Some(custom) = custom.as_ref() {
        for (k, v) in custom {
            map.insert(k, v);
        }
    }

    serialize_data_field(state, "custom", &Some(map))
}
