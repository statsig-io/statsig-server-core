use crate::evaluation::dynamic_value::DynamicValue;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

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
