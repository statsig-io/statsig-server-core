use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{log_e, user::StatsigUserInternal};

use super::statsig_user_internal::FullUserKey;

const TAG: &str = "StatsigUserLoggable";

lazy_static::lazy_static! {
    static ref LOGGABLE_USER_STORE: RwLock<HashMap<FullUserKey, Weak<UserLoggableData>>> =
    RwLock::new(HashMap::new());
}

#[derive(Serialize, Deserialize)]
pub struct UserLoggableData {
    pub key: Option<FullUserKey>,
    pub value: Value,
}

#[derive(Clone)]
pub struct StatsigUserLoggable {
    pub data: Arc<UserLoggableData>,
}

impl Serialize for StatsigUserLoggable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StatsigUserLoggable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(StatsigUserLoggable {
            data: Arc::new(UserLoggableData { key: None, value }),
        })
    }
}

fn make_loggable(
    full_user_key: FullUserKey,
    user_internal: &StatsigUserInternal,
) -> Arc<UserLoggableData> {
    let result = Arc::new(UserLoggableData {
        key: Some(full_user_key.clone()),
        value: json!(user_internal),
    });

    let mut store = match LOGGABLE_USER_STORE.write() {
        Ok(store) => store,
        Err(e) => {
            log_e!(TAG, "Error locking user loggable store: {:?}", e);
            return result;
        }
    };

    store.insert(full_user_key, Arc::downgrade(&result));

    result
}

impl StatsigUserLoggable {
    pub fn new(user_internal: &StatsigUserInternal) -> Self {
        let user_key = user_internal.get_full_user_key();

        let mut existing = None;

        match LOGGABLE_USER_STORE.read() {
            Ok(store) => existing = store.get(&user_key).map(|x| x.upgrade()),
            Err(e) => {
                log_e!(TAG, "Error locking user loggable store: {:?}", e);
            }
        };

        let data = match existing {
            Some(Some(x)) => x,
            _ => make_loggable(user_key, user_internal),
        };

        Self { data }
    }

    pub fn null_user() -> Self {
        Self {
            data: Arc::new(UserLoggableData {
                key: None,
                value: Value::Null,
            }),
        }
    }

    pub fn create_sampling_key(&self) -> String {
        let user_data = &self.data.value;
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

impl Drop for StatsigUserLoggable {
    fn drop(&mut self) {
        let full_user_key = match &self.data.key {
            Some(k) => k,
            None => return,
        };

        let strong_count = match LOGGABLE_USER_STORE.read() {
            Ok(store) => match store.get(full_user_key) {
                Some(weak_ref) => weak_ref.strong_count(),
                None => return,
            },
            Err(e) => {
                log_e!(TAG, "Error locking user loggable store: {:?}", e);
                return;
            }
        };

        if strong_count > 1 {
            return;
        }

        match LOGGABLE_USER_STORE.write() {
            Ok(mut store) => {
                store.remove(full_user_key);
            }
            Err(e) => {
                log_e!(TAG, "Error locking user loggable store: {:?}", e);
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::future::join_all;

    use crate::StatsigUser;

    use super::*;

    #[tokio::test]
    async fn test_creating_many_loggable_users_and_map_growth() {
        let mut handles = vec![];

        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                for i in 0..1000 {
                    let user_data = StatsigUser::with_user_id(format!("user{}", i));
                    let user_internal = StatsigUserInternal::new(&user_data, None);
                    let loggable = user_internal.to_loggable();
                    tokio::time::sleep(Duration::from_micros(1)).await;
                    let _ = loggable; // held across the sleep
                }
            });

            handles.push(handle);
        }

        join_all(handles).await;

        assert_eq!(LOGGABLE_USER_STORE.read().unwrap().len(), 0);
    }
}
