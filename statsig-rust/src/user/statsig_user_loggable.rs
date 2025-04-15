use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::StatsigUserInternal;

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

impl StatsigUserInternal<'_, '_> {
    pub fn to_loggable(&self) -> StatsigUserLoggable {
        StatsigUserLoggable::new(self)
    }
}
