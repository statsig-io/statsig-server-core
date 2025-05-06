use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::log_event_payload::{LogEventPayload, LogEventRequest};
use crate::statsig_metadata::StatsigMetadataWithLogEventExtras;
use serde_json::json;

pub struct EventBatch {
    pub attempts: u8,
    pub events: Vec<StatsigEventInternal>,
}

impl EventBatch {
    pub fn new(events: Vec<StatsigEventInternal>) -> Self {
        Self {
            events,
            attempts: 0,
        }
    }

    pub fn get_log_event_request(
        &self,
        statsig_metadata: StatsigMetadataWithLogEventExtras,
    ) -> LogEventRequest {
        let payload = LogEventPayload {
            events: json!(self.events),
            statsig_metadata: json!(statsig_metadata),
        };

        LogEventRequest {
            payload,
            event_count: self.events.len() as u64,
            retries: self.attempts as u32,
        }
    }
}
