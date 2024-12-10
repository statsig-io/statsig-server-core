use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use crate::{event_logging::statsig_event_internal::StatsigEventInternal, log_e};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventPayload {
    pub events: Value,
    pub statsig_metadata: Value,
}

const TAG: &str = stringify!(LogEventPayload);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventRequest {
    pub payload: LogEventPayload,
    pub event_count: u64,
}

impl LogEventRequest {
    pub fn merge(&mut self, other: &LogEventRequest) {
        self.event_count += other.event_count;

        let mut right: Vec<StatsigEventInternal> = match from_value(self.payload.events.clone()) {
            Ok(events) => events,
            Err(e) => {
                log_e!(TAG, "Failed to parse events: {}", e);
                return;
            }
        };

        let left: Vec<StatsigEventInternal> = match from_value(other.payload.events.clone()) {
            Ok(events) => events,
            Err(e) => {
                log_e!(TAG, "Failed to parse events: {}", e);
                return;
            }
        };

        right.extend(left);

        self.payload.events = json!(right);
    }
}
