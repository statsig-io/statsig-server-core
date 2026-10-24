use amplitude_rust::{Amplitude, Options};
use async_trait::async_trait;
use serde_json::{json, Value};
use statsig_amplitude::StatsigAmplitudeAdapter;
use statsig_rust::{
    log_event_payload::{LogEventPayload, LogEventRequest},
    EventLoggingAdapter, StatsigErr, StatsigRuntime,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

struct Primary {
    calls: AtomicUsize,
    success: bool,
}

#[allow(clippy::double_must_use)]
#[async_trait]
impl EventLoggingAdapter for Primary {
    async fn start(&self, _: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }
    async fn log_events(&self, _: LogEventRequest) -> Result<bool, StatsigErr> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.success)
    }
    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }
    fn should_schedule_background_flush(&self) -> bool {
        true
    }
}

fn request() -> LogEventRequest {
    LogEventRequest {
        payload: LogEventPayload {
            events: json!([
                {"eventName":"purchase", "time":123456789, "value":42,
                 "metadata":{"item":"book"}, "user":{"userID":"user-123",
                 "email":"private@example.com", "privateAttributes":{"secret":"hidden"}}},
                {"eventName":"statsig::config_exposure", "time":123456790,
                 "metadata":{"config":"checkout", "ruleID":"rule-1"},
                 "secondaryExposures":[{"gate":"holdout", "gateValue":"false", "ruleID":"rule-2"}],
                 "user":{"userID":"user-123"}},
                {"eventName":"statsig::diagnostics", "time":123456791, "user":{}}
            ]),
            statsig_metadata: json!({"sdkType":"statsig-server-core"}),
        },
        event_count: 3,
        retries: 0,
    }
}

#[tokio::test]
async fn forwards_custom_events_and_exposures_alongside_primary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let amplitude = Amplitude::new(
        "amplitude-key",
        Options {
            server_url: Some(server.uri()),
            ..Options::default()
        },
    )
    .unwrap();
    let primary = Arc::new(Primary {
        calls: AtomicUsize::new(0),
        success: true,
    });
    let adapter = StatsigAmplitudeAdapter::new(primary.clone(), amplitude);
    assert!(adapter.log_events(request()).await.unwrap());
    assert_eq!(primary.calls.load(Ordering::SeqCst), 1);
    assert!(adapter.should_schedule_background_flush());
    let requests = server.received_requests().await.unwrap();
    let body: Value = requests[0].body_json().unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
    assert_eq!(body["events"][0]["event_type"], "purchase");
    assert_eq!(
        body["events"][0]["event_properties"]["statsig"]["value"],
        42
    );
    assert_eq!(body["events"][1]["event_type"], "statsig::config_exposure");
    assert_eq!(
        body["events"][1]["event_properties"]["statsig"]["secondary_exposures"][0]["gate"],
        "holdout"
    );
    assert!(!String::from_utf8(requests[0].body.clone())
        .unwrap()
        .contains("private@example.com"));
    assert!(!String::from_utf8(requests[0].body.clone())
        .unwrap()
        .contains("hidden"));
    adapter.shutdown().await.unwrap();
}

#[test]
fn retry_ids_are_stable_across_object_order_and_request_metadata_changes() {
    let first = request();
    let mut second = first.clone();
    second.retries = 10;
    second.payload.statsig_metadata = json!({"changed":true});
    second.payload.events[0]["metadata"] = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
    let mut reordered = second.clone();
    reordered.payload.events[0]["metadata"] = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
    assert_eq!(
        StatsigAmplitudeAdapter::map_events(&first).unwrap()[0].insert_id,
        StatsigAmplitudeAdapter::map_events(&request()).unwrap()[0].insert_id
    );
    assert_eq!(
        StatsigAmplitudeAdapter::map_events(&second).unwrap()[0].insert_id,
        StatsigAmplitudeAdapter::map_events(&reordered).unwrap()[0].insert_id
    );
    let mut metadata_only = first.clone();
    metadata_only.retries = 1;
    metadata_only.payload.statsig_metadata = json!({"changed":true});
    assert_eq!(
        StatsigAmplitudeAdapter::map_events(&first).unwrap()[0].insert_id,
        StatsigAmplitudeAdapter::map_events(&metadata_only).unwrap()[0].insert_id
    );
}

#[tokio::test]
async fn amplitude_failure_does_not_retry_successful_statsig_upload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&server)
        .await;
    let amplitude = Amplitude::new(
        "key",
        Options {
            server_url: Some(server.uri()),
            ..Options::default()
        },
    )
    .unwrap();
    let primary = Arc::new(Primary {
        calls: AtomicUsize::new(0),
        success: true,
    });
    let adapter = StatsigAmplitudeAdapter::new(primary.clone(), amplitude);
    assert!(adapter.log_events(request()).await.unwrap());
    assert_eq!(primary.calls.load(Ordering::SeqCst), 1);
    adapter.shutdown().await.unwrap();
}

#[tokio::test]
async fn unsuccessful_primary_upload_is_not_forwarded() {
    let server = MockServer::start().await;
    let amplitude = Amplitude::new(
        "key",
        Options {
            server_url: Some(server.uri()),
            ..Options::default()
        },
    )
    .unwrap();
    let primary = Arc::new(Primary {
        calls: AtomicUsize::new(0),
        success: false,
    });
    let adapter = StatsigAmplitudeAdapter::new(primary, amplitude);
    assert!(!adapter.log_events(request()).await.unwrap());
    assert!(server.received_requests().await.unwrap().is_empty());
    adapter.shutdown().await.unwrap();
}
