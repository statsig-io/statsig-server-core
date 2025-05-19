use crate::{evaluation::dynamic_value::DynamicValue, hashing};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

const EMPTY_HASHES: &[u64] = &[0];

#[skip_serializing_none]
#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(rename = "userID")]
    pub user_id: Option<DynamicValue>,
    #[serde(rename = "customIDs")]
    pub custom_ids: Option<HashMap<String, DynamicValue>>,

    pub email: Option<DynamicValue>,
    pub ip: Option<DynamicValue>,
    pub user_agent: Option<DynamicValue>,
    pub country: Option<DynamicValue>,
    pub locale: Option<DynamicValue>,
    pub app_version: Option<DynamicValue>,

    #[serde(skip_serializing)]
    pub private_attributes: Option<HashMap<String, DynamicValue>>,
    pub custom: Option<HashMap<String, DynamicValue>>,
}

impl UserData {
    pub fn create_user_values_hash(&self) -> u64 {
        let hashes = self.get_all_user_hashes();
        hashing::hash_one(hashes)
    }

    fn get_all_user_hashes(&self) -> Vec<u64> {
        let mut hashes = Vec::new();
        push_string_field_hashes(&mut hashes, &self.user_id);
        push_map_field_hashes(&mut hashes, &self.custom_ids);

        push_string_field_hashes(&mut hashes, &self.app_version);
        push_string_field_hashes(&mut hashes, &self.country);
        push_string_field_hashes(&mut hashes, &self.email);
        push_string_field_hashes(&mut hashes, &self.ip);
        push_string_field_hashes(&mut hashes, &self.locale);
        push_string_field_hashes(&mut hashes, &self.user_agent);

        push_map_field_hashes(&mut hashes, &self.custom);
        push_map_field_hashes(&mut hashes, &self.private_attributes);

        hashes
    }
}

fn push_string_field_hashes(hashes: &mut Vec<u64>, field: &Option<DynamicValue>) {
    if let Some(field) = field {
        hashes.push(field.hash_value);
    } else {
        hashes.push(0);
    }
}

fn push_map_field_hashes(hashes: &mut Vec<u64>, field: &Option<HashMap<String, DynamicValue>>) {
    if let Some(field) = field {
        hashes.extend(field.values().map(|id| id.hash_value));
    } else {
        hashes.extend(EMPTY_HASHES);
    }
}
