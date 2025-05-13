use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventPayload {
    pub events: Value,
    pub statsig_metadata: Value,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogEventRequest {
    pub payload: LogEventPayload,
    pub event_count: u64,
    pub retries: u32,
}
