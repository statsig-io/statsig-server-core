use crate::{evaluation::dynamic_value::DynamicValue, hashing};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

pub type UserDataMap = IndexMap<String, DynamicValue>;

pub trait IntoUserDataMap {
    fn into_user_data_map(self) -> UserDataMap;
}

pub trait IntoOptionalUserDataMap {
    fn into_optional_user_data_map(self) -> Option<UserDataMap>;
}

impl<K, V> IntoUserDataMap for HashMap<K, V>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_user_data_map(self) -> UserDataMap {
        self.into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

impl<K, V> IntoOptionalUserDataMap for HashMap<K, V>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        Some(self.into_user_data_map())
    }
}

impl<K, V> IntoOptionalUserDataMap for Option<HashMap<K, V>>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        self.map(IntoUserDataMap::into_user_data_map)
    }
}

impl<K, V> IntoUserDataMap for IndexMap<K, V>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_user_data_map(self) -> UserDataMap {
        self.into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

impl<K, V> IntoOptionalUserDataMap for IndexMap<K, V>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        Some(self.into_user_data_map())
    }
}

impl<K, V> IntoOptionalUserDataMap for Option<IndexMap<K, V>>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        self.map(IntoUserDataMap::into_user_data_map)
    }
}

impl<K, V> IntoUserDataMap for Vec<(K, V)>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_user_data_map(self) -> UserDataMap {
        self.into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

impl<K, V> IntoOptionalUserDataMap for Vec<(K, V)>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        Some(self.into_user_data_map())
    }
}

impl<K, V> IntoOptionalUserDataMap for Option<Vec<(K, V)>>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        self.map(IntoUserDataMap::into_user_data_map)
    }
}

impl<K, V, const N: usize> IntoUserDataMap for [(K, V); N]
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_user_data_map(self) -> UserDataMap {
        self.into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

impl<K, V, const N: usize> IntoOptionalUserDataMap for [(K, V); N]
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        Some(self.into_user_data_map())
    }
}

impl<K, V, const N: usize> IntoOptionalUserDataMap for Option<[(K, V); N]>
where
    K: Into<String>,
    V: Into<DynamicValue>,
{
    fn into_optional_user_data_map(self) -> Option<UserDataMap> {
        self.map(IntoUserDataMap::into_user_data_map)
    }
}

#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(rename = "userID")]
    pub user_id: Option<DynamicValue>,
    #[serde(rename = "customIDs")]
    pub custom_ids: Option<UserDataMap>,

    pub email: Option<DynamicValue>,
    pub ip: Option<DynamicValue>,
    pub user_agent: Option<DynamicValue>,
    pub country: Option<DynamicValue>,
    pub locale: Option<DynamicValue>,
    pub app_version: Option<DynamicValue>,
    pub statsig_environment: Option<UserDataMap>,

    #[serde(skip_serializing)]
    pub private_attributes: Option<UserDataMap>,
    pub custom: Option<UserDataMap>,
}

impl UserData {
    pub fn create_exposure_dedupe_user_hash(&self, unit_id_type: Option<&str>) -> u64 {
        let user_id_hash = self
            .user_id
            .as_ref()
            .map_or(0, |user_id| user_id.hash_value);
        let stable_id_hash = self.get_unit_id_hash("stableID");
        let unit_type_hash = unit_id_type.map_or(0, |id_type| self.get_unit_id_hash(id_type));
        let custom_ids_hash_sum = self.sum_custom_id_hashes();

        hashing::hash_one(vec![
            user_id_hash,
            stable_id_hash,
            unit_type_hash,
            custom_ids_hash_sum,
        ])
    }

    pub fn sum_custom_id_hashes(&self) -> u64 {
        self.custom_ids.as_ref().map_or(0, |custom_ids| {
            custom_ids
                .values()
                .fold(0u64, |acc, value| acc.wrapping_add(value.hash_value))
        })
    }

    pub fn get_unit_id_hash(&self, id_type: &str) -> u64 {
        if id_type.eq_ignore_ascii_case("userid") {
            return self
                .user_id
                .as_ref()
                .map_or(0, |user_id| user_id.hash_value);
        }

        if let Some(custom_ids) = &self.custom_ids {
            if let Some(id) = custom_ids.get(id_type) {
                return id.hash_value;
            }

            if let Some(id) = custom_ids.get(&id_type.to_lowercase()) {
                return id.hash_value;
            }
        }

        0
    }

    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        serde_json::to_vec(self).ok()
    }
}
