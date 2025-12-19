use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub fn string_metadata_to_value_metadata(
    metadata: HashMap<String, String>,
) -> HashMap<String, Value> {
    metadata
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigEvent {
    pub event_name: String,
    pub value: Option<Value>,
    pub metadata: Option<HashMap<String, Value>>,
    pub statsig_metadata: Option<HashMap<String, Value>>,
}
