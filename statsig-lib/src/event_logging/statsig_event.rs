use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigEvent {
    pub event_name: String,
    pub value: Option<Value>,
    pub metadata: Option<HashMap<String, String>>,
}
