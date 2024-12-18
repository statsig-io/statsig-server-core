use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use crate::{event_logging::statsig_event_internal::StatsigEventInternal, StatsigErr};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventPayload {
    pub events: Value,
    pub statsig_metadata: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventRequest {
    pub payload: LogEventPayload,
    pub event_count: u64,
}

impl LogEventRequest {
    pub fn merge(&mut self, other: &LogEventRequest) -> Result<(), StatsigErr> {
        let right = parse_events(&self.payload.events)?;
        let left = parse_events(&other.payload.events)?;

        let mut expo_keys = HashSet::new();
        let mut merged_events = vec![];

        merge_event_into(right, &mut expo_keys, &mut merged_events);
        merge_event_into(left, &mut expo_keys, &mut merged_events);

        self.payload.events = json!(merged_events);
        self.event_count = merged_events.len() as u64;

        Ok(())
    }
}

fn parse_events(events: &Value) -> Result<Vec<StatsigEventInternal>, StatsigErr> {
    match from_value(events.clone()) {
        Ok(events) => Ok(events),
        Err(e) => Err(StatsigErr::JsonParseError(stringify!(Vec<StatsigEventInternal>).to_string(), e.to_string()))
    }
}

fn merge_event_into(
    events: Vec<StatsigEventInternal>,
    expo_keys: &mut HashSet<String>,
    merged_events: &mut Vec<StatsigEventInternal>,
) {
    for event in events {
        if event.is_exposure_event() {
            let key = create_merge_key(&event);
            if expo_keys.contains(&key) {
                continue;
            }
            expo_keys.insert(key);
        }

        merged_events.push(event);
    }
}

fn create_merge_key(event: &StatsigEventInternal) -> String {
    let mut metadata_string = String::new();
    if let Some(metadata) = &event.event_data.metadata {
        metadata_string = metadata
            .values()
            .cloned()
            .collect::<Vec<String>>()
            .join(",");
    }

    format!(
        "{}|{}|{}",
        event.event_data.event_name, event.user.value, metadata_string
    )
}
