use std::sync::Arc;

use crate::log_event_payload::LogEventRequest;
use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;

#[async_trait]
pub trait EventLoggingAdapter: Send + Sync {
    async fn start(&self, statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr>;
    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr>;
    async fn shutdown(&self) -> Result<(), StatsigErr>;
    fn should_schedule_background_flush(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::event_logging::statsig_event::StatsigEvent;
    use crate::event_logging::statsig_event_internal::make_custom_event;
    use crate::log_event_payload::{LogEventPayload, LogEventRequest};
    use crate::statsig_metadata::StatsigMetadata;
    use crate::statsig_user_internal::StatsigUserInternal;
    use crate::StatsigUser;

    #[test]
    fn test_request_jsonify() {
        let user = StatsigUser::with_user_id("a-user".to_string());
        let user_internal = StatsigUserInternal::new(&user, None);
        let event = StatsigEvent {
            event_name: "my_custom_event".to_string(),
            value: None,
            metadata: None,
            statsig_metadata: None,
        };

        let event = make_custom_event(user_internal, event);

        let payload = LogEventPayload {
            events: json!([event]),
            statsig_metadata: StatsigMetadata::get_as_json(),
        };

        let request = LogEventRequest {
            payload,
            event_count: 1,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        println!("{}", serialized);

        let deserialized: Value = serde_json::from_str(&serialized).unwrap();

        let event_count = deserialized.get("eventCount").unwrap().as_u64().unwrap();
        assert_eq!(event_count, 1);

        let payload_json = deserialized.get("payload").unwrap();

        let statsig_metadata_json = payload_json
            .get("statsigMetadata")
            .unwrap()
            .as_object()
            .unwrap();

        assert!(statsig_metadata_json.contains_key("sdkType"));
        assert!(statsig_metadata_json.contains_key("sdkVersion"));
        assert!(statsig_metadata_json.contains_key("sessionID"));

        let events_json = payload_json.get("events").unwrap();
        assert_eq!(events_json.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_json_deserialization() {
        let request_json = r#"{"payload":{"events":[{"eventName":"my_custom_event","metadata":null,"secondaryExposures":null,"time":1730831508904,"user":{"statsigEnvironment":null,"userID":"a-user"},"value":null}],"statsigMetadata":{"sdkType":"statsig-server-core","sdkVersion":"0.0.1","sessionId":"1ff863ed-a9ab-4785-bb0e-1a7b0140c040"}},"eventCount":1}"#;
        let request: LogEventRequest = serde_json::from_str(request_json).unwrap();
        assert_eq!(request.event_count, 1);
    }
}
