use serde_json::json;
use statsig_rust::{Amplitude, AmplitudeError, AmplitudeEvent, AmplitudeOptions};
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn init_and_track() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/httpapi"))
        .and(body_partial_json(json!({
            "api_key": "test-api-key",
            "events": [{
                "event_type": "Song Played",
                "user_id": "user-123",
                "event_properties": {"title": "Hello", "repeat": true},
                "time": 1234567890000_i64,
                "insert_id": "event-123"
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"code": 200})))
        .expect(1)
        .mount(&server)
        .await;

    let amplitude = Amplitude::new();
    amplitude
        .init(
            "test-api-key",
            Some(AmplitudeOptions {
                server_url: format!("{}/2/httpapi", server.uri()),
                ..AmplitudeOptions::default()
            }),
        )
        .unwrap();
    amplitude
        .track(AmplitudeEvent {
            user_id: Some("user-123".into()),
            event_properties: Some(
                json!({"title": "Hello", "repeat": true})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
            time: Some(1234567890000),
            insert_id: Some("event-123".into()),
            ..AmplitudeEvent::new("Song Played")
        })
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(body["events"][0]["library"]
        .as_str()
        .unwrap()
        .starts_with("amplitude-rust-core/"));
    assert!(!requests[0].headers.contains_key("statsig-api-key"));
    assert!(!requests[0].headers.contains_key("statsig-client-time"));
}

#[tokio::test]
async fn track_reports_errors_and_populates_defaults() {
    let amplitude = Amplitude::new();
    assert!(matches!(
        amplitude.track(AmplitudeEvent::new("test")).await,
        Err(AmplitudeError::NotInitialized)
    ));
    assert!(amplitude.init("", None).is_err());

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "Invalid API key"})))
        .expect(1)
        .mount(&server)
        .await;
    amplitude
        .init(
            "test-api-key",
            Some(AmplitudeOptions {
                server_url: server.uri(),
                ..AmplitudeOptions::default()
            }),
        )
        .unwrap();
    assert!(amplitude.track(AmplitudeEvent::new("test")).await.is_err());
    let result = amplitude
        .track(AmplitudeEvent {
            device_id: Some("device-123".into()),
            ..AmplitudeEvent::new("test")
        })
        .await;
    assert!(matches!(
        result,
        Err(AmplitudeError::Api { status: 400, .. })
    ));
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(body["events"][0]["time"].as_i64().unwrap() > 0);
    assert!(uuid::Uuid::parse_str(body["events"][0]["insert_id"].as_str().unwrap()).is_ok());
}

use statsig_rust::{
    AmplitudeCallback, AmplitudeIngestionMetadata, AmplitudeLogMode, AmplitudeMiddleware,
    AmplitudeMiddlewareNext, AmplitudeMiddlewarePayload, AmplitudePlan, AmplitudeResponse,
    AmplitudeStatus, AMPLITUDE_API_URL, AMPLITUDE_BATCH_API_URL, AMPLITUDE_RETRY_TIMEOUTS,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

fn event(name: &str) -> AmplitudeEvent {
    AmplitudeEvent {
        user_id: Some("user-123".into()),
        ..AmplitudeEvent::new(name)
    }
}
fn client(server: &MockServer) -> Amplitude {
    let client = Amplitude::new();
    client
        .init(
            "key",
            Some(AmplitudeOptions {
                server_url: server.uri(),
                ..Default::default()
            }),
        )
        .unwrap();
    client.set_log_mode(AmplitudeLogMode::Off);
    client
}
type Responses = Arc<Mutex<Vec<(String, u16, String)>>>;
fn record() -> (Responses, AmplitudeCallback) {
    let responses = Arc::new(Mutex::new(Vec::new()));
    let result = responses.clone();
    (
        responses,
        Arc::new(move |event, code, message| {
            result
                .lock()
                .unwrap()
                .push((event.event_type.clone(), code, message.into()))
        }),
    )
}

#[test]
fn java_event_fields_and_defaults() {
    let mut event = event("full");
    event.time = Some(123);
    event.insert_id = Some("insert".into());
    event.location_lat = 12.5;
    event.location_lng = -13.5;
    event.device_brand = Some("brand".into());
    event.android_id = Some("android".into());
    event.partner_id = Some("partner".into());
    event.event_id = 7;
    event.price = Some(2.5);
    event.quantity = 0;
    event.revenue = Some(10.0);
    event.product_id = Some("sku".into());
    event.revenue_type = Some("purchase".into());
    event.currency = Some("USD".into());
    event.plan = Some(AmplitudePlan {
        version_id: Some("version-id".into()),
        source: Some(String::new()),
        ..Default::default()
    });
    event.ingestion_metadata = Some(AmplitudeIngestionMetadata {
        source_name: Some("ampli".into()),
        ..Default::default()
    });
    let json = event.to_json_object().unwrap();
    assert_eq!(json["time"], 123);
    assert_eq!(json["location_lat"], 12.5);
    assert_eq!(json["device_id"], json!(null));
    assert_eq!(json["device_brand"], "brand");
    assert_eq!(json["android_id"], "android");
    assert_eq!(json["partner_id"], "partner");
    assert_eq!(json["quantity"], 1);
    assert_eq!(json["productId"], "sku");
    assert_eq!(json["revenueType"], "purchase");
    assert_eq!(json["event_id"], 7);
    assert_eq!(json["session_id"], -1);
    assert!(json.get("app_version").is_none());
    assert_eq!(json["groups"], json!({}));
    assert_eq!(json["event_properties"], json!({}));
    assert_eq!(json["plan"], json!({"versionId":"version-id"}));
    assert_eq!(json["ingestion_metadata"], json!({"source_name":"ampli"}));
    assert!(AmplitudeEvent::with_identity("invalid", None, None).is_err());
    let plain = event::plain();
    assert!(plain.get("price").is_none());
    assert!(plain.get("currency").is_none());
}
mod event {
    pub fn plain() -> serde_json::Value {
        super::event("plain").to_json_object().unwrap()
    }
}

#[test]
fn property_truncation_and_map_helpers_follow_java() {
    let mut event = event("properties");
    event.add_event_property("long", json!("a".repeat(1001)));
    event.add_user_property("nested", json!({"array":["x".repeat(1001)]}));
    event.add_group("org", json!("amplitude"));
    event.add_group_property("size", json!(42));
    event
        .add_event_property("remove", json!(1))
        .add_event_property("remove", json!(null));
    let value = event.to_json_object().unwrap();
    assert_eq!(
        value["event_properties"]["long"].as_str().unwrap().len(),
        1000
    );
    assert_eq!(
        value["user_properties"]["nested"]["array"][0]
            .as_str()
            .unwrap()
            .len(),
        1000
    );
    assert!(value["event_properties"].get("remove").is_none());
    let properties = (0..1025).map(|i| (i.to_string(), json!(i))).collect();
    event.set_event_properties(Some(properties));
    assert!(event.to_json_object().is_err());
}

#[test]
fn named_instances_and_endpoint_precedence() {
    let a = Amplitude::get_instance(None);
    let b = Amplitude::get_instance(Some(""));
    assert!(a.is_same_instance(&b));
    let c = Amplitude::get_instance(Some("other"));
    assert!(!a.is_same_instance(&c));
    a.init("key", None).unwrap();
    assert_eq!(a.server_url(), AMPLITUDE_API_URL);
    a.use_batch_mode(true);
    assert_eq!(a.server_url(), AMPLITUDE_BATCH_API_URL);
    a.init("key", None).unwrap();
    assert_eq!(a.server_url(), AMPLITUDE_API_URL);
    a.set_server_url("https://proxy.example/events")
        .use_batch_mode(true);
    a.set_server_url("invalid");
    assert_eq!(a.server_url(), "https://proxy.example/events");
}

#[tokio::test]
async fn buffering_and_default_metadata_and_custom_headers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    client
        .set_event_upload_threshold(2)
        .set_plan(Some(AmplitudePlan {
            source: Some("default".into()),
            ..Default::default()
        }));
    let mut options = AmplitudeOptions::default();
    options
        .set_min_id_length(Some(1))
        .add_header("X-Test", "custom");
    client.set_options(Some(options));
    client.log_event(event("one")).await.unwrap();
    assert!(server.received_requests().await.unwrap().is_empty());
    let mut override_event = event("two");
    override_event.plan = Some(AmplitudePlan {
        source: Some("override".into()),
        ..Default::default()
    });
    client.log_event(override_event).await.unwrap();
    client.flush_events_and_wait().await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
    assert_eq!(body["options"], json!({"min_id_length":1}));
    assert_eq!(body["events"][0]["plan"]["source"], "default");
    assert_eq!(body["events"][1]["plan"]["source"], "override");
    assert_eq!(requests[0].headers["x-test"], "custom");
    assert_eq!(responses.lock().unwrap().len(), 2);
    client.shutdown().await.unwrap();
}

struct Middleware {
    order: Arc<Mutex<Vec<String>>>,
    name: &'static str,
    drop: bool,
}
#[allow(clippy::double_must_use)]
#[async_trait::async_trait]
impl AmplitudeMiddleware for Middleware {
    async fn run(
        &self,
        mut payload: AmplitudeMiddlewarePayload,
        next: AmplitudeMiddlewareNext,
    ) -> Result<Option<AmplitudeMiddlewarePayload>, AmplitudeError> {
        self.order
            .lock()
            .unwrap()
            .push(format!("before-{}", self.name));
        payload
            .event
            .add_event_property(self.name, payload.extra.clone());
        let result = if self.drop {
            None
        } else {
            next.run(payload).await?
        };
        self.order
            .lock()
            .unwrap()
            .push(format!("after-{}", self.name));
        Ok(result)
    }
}
#[tokio::test]
async fn middleware_continuations_order_and_drop() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    let order = Arc::new(Mutex::new(Vec::new()));
    for name in ["a", "b"] {
        client.add_event_middleware(Arc::new(Middleware {
            order: order.clone(),
            name,
            drop: false,
        }));
    }
    client
        .log_event_with(event("middleware"), None, json!({"request":1}))
        .await
        .unwrap();
    client.flush_events_and_wait().await.unwrap();
    assert_eq!(
        *order.lock().unwrap(),
        vec!["before-a", "before-b", "after-b", "after-a"]
    );
    client.add_event_middleware(Arc::new(Middleware {
        order,
        name: "drop",
        drop: true,
    }));
    client.log_event(event("dropped")).await.unwrap();
    client.flush_events_and_wait().await.unwrap();
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_indices_drop_only_bad_events_before_grouped_retry() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    Mock::given(method("POST")).respond_with(move |_:&wiremock::Request| {
        if count.fetch_add(1,Ordering::SeqCst)==0 { ResponseTemplate::new(400).set_body_json(json!({"code":400,"error":"bad field","events_with_invalid_fields":{"event_type":[1,1]}})) }
        else { ResponseTemplate::new(200) }
    }).expect(2).mount(&server).await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    for name in ["one", "bad", "three"] {
        client.log_event(event(name)).await.unwrap();
    }
    client.flush_events_and_wait().await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[1].body_json().unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
    let responses = responses.lock().unwrap().clone();
    assert_eq!(responses.len(), 3);
    assert!(responses
        .iter()
        .any(|(name, code, _)| name == "bad" && *code == 400));
    assert_eq!(
        responses.iter().filter(|(_, code, _)| *code == 200).count(),
        2
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn payload_reduction_matches_java_instead_of_splitting_and_resending_both_halves() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            ResponseTemplate::new(if calls.fetch_add(1, Ordering::SeqCst) < 2 {
                413
            } else {
                200
            })
        })
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    for name in ["a", "b", "c", "d"] {
        client.log_event(event(name)).await.unwrap();
    }
    client.flush_events_and_wait().await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let counts: Vec<_> = requests
        .iter()
        .map(|r| {
            r.body_json::<serde_json::Value>().unwrap()["events"]
                .as_array()
                .unwrap()
                .len()
        })
        .collect();
    assert_eq!(counts, vec![4, 4, 2]);
    assert_eq!(
        responses
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, code, _)| *code == 413)
            .count(),
        2
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn rate_limit_quota_filter_and_exact_throttled_id_tracking() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    Mock::given(method("POST")).respond_with(move |_:&wiremock::Request| {
        if calls.fetch_add(1,Ordering::SeqCst)==0 { ResponseTemplate::new(429).set_body_json(json!({"code":429,"error":"rate limited","eps_threshold":30,"throttled_users":{"user-123":2},"exceeded_daily_quota_users":{"quota-user":1}})) }else{ResponseTemplate::new(200)}
    }).expect(2).mount(&server).await;
    let client = client(&server);
    client.set_record_throttled_id(true);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    client.log_event(event("retry")).await.unwrap();
    let mut quota = event("quota");
    quota.user_id = Some("quota-user".into());
    client.log_event(quota).await.unwrap();
    client.flush_events().unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !client.should_wait(&event("same-user")) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut other = event("other");
    other.user_id = Some("other-user".into());
    assert!(!client.should_wait(&other));
    client.flush_events_and_wait().await.unwrap();
    assert!(!client.should_wait(&event("same-user")));
    assert!(responses
        .lock()
        .unwrap()
        .iter()
        .any(|(name, code, _)| name == "quota" && *code == 429));
    client.shutdown().await.unwrap();
}

#[test]
fn response_status_body_and_invalid_key_match_java() {
    for (code, status) in [
        (0, AmplitudeStatus::Unknown),
        (201, AmplitudeStatus::Success),
        (408, AmplitudeStatus::Timeout),
        (429, AmplitudeStatus::Ratelimit),
        (413, AmplitudeStatus::PayloadTooLarge),
        (403, AmplitudeStatus::Invalid),
        (500, AmplitudeStatus::Failed),
    ] {
        assert_eq!(
            AmplitudeResponse::parse(code, "<html>").unwrap().status,
            status
        );
        assert_eq!(AmplitudeResponse::parse(code, "").unwrap().status, status);
    }
    assert!(matches!(
        AmplitudeResponse::parse(400, r#"{"code":400,"error":"Invalid API key: key"}"#),
        Err(AmplitudeError::InvalidApiKey)
    ));
    assert_eq!(
        AMPLITUDE_RETRY_TIMEOUTS,
        [100, 100, 200, 200, 400, 400, 800, 800, 1600, 1600, 3200, 3200]
    );
}

#[tokio::test]
async fn shutdown_callbacks_and_execution_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(500)))
        .expect(0..=1)
        .mount(&server)
        .await;
    let client = client(&server);
    client.set_flush_timeout(10);
    let result = client.track(event("timeout")).await;
    assert!(matches!(result, Err(AmplitudeError::Api { status: 0, .. })));
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    client.log_event(event("buffered")).await.unwrap();
    client.shutdown().await.unwrap();
    assert_eq!(responses.lock().unwrap()[0].1, 0);
    assert!(matches!(
        client.log_event(event("late")).await,
        Err(AmplitudeError::Shutdown)
    ));
}

#[tokio::test]
async fn period_flush_sends_buffered_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    client.set_event_upload_period_millis(10);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    client.log_event(event("period")).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while responses.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn full_retry_schedule_is_bounded_and_calls_back_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .expect(13)
        .mount(&server)
        .await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    client.log_event(event("exhaust")).await.unwrap();
    client.flush_events_and_wait().await.unwrap();
    let responses = responses.lock().unwrap().clone();
    assert_eq!(
        responses,
        vec![("exhaust".into(), 503, "Event retries exhausted.".into())]
    );
    let requests = server.received_requests().await.unwrap();
    assert!(requests.windows(2).all(|pair| pair[0].body == pair[1].body));
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_events_are_filtered_during_retry() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    Mock::given(method("POST")).respond_with(move |_: &wiremock::Request| {
        match calls.fetch_add(1, Ordering::SeqCst) {
            0 => ResponseTemplate::new(503),
            1 => ResponseTemplate::new(400).set_body_json(json!({"code":400,"error":"bad field","events_with_missing_fields":{"event_type":[1]}})),
            _ => ResponseTemplate::new(200),
        }
    }).expect(3).mount(&server).await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    for name in ["one", "bad", "three"] {
        client.log_event(event(name)).await.unwrap();
    }
    client.flush_events_and_wait().await.unwrap();
    let responses = responses.lock().unwrap().clone();
    assert_eq!(responses.len(), 3);
    assert!(responses.iter().any(|(name, status, message)| name == "bad"
        && *status == 400
        && message == "Invalid events."));
    assert_eq!(
        responses
            .iter()
            .filter(|(_, status, _)| *status == 200)
            .count(),
        2
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_cancels_pending_retries_and_clears_throttled_ids() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(429).set_body_json(json!({"code":429,"error":"throttled","eps_threshold":30,"throttled_users":{"user-123":1}}))).expect(1).mount(&server).await;
    let client = client(&server);
    let (responses, callback) = record();
    client
        .set_callbacks(Some(callback))
        .set_record_throttled_id(true);
    client.log_event(event("cancel")).await.unwrap();
    client.flush_events().unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !client.should_wait(&event("same-user")) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    client.shutdown().await.unwrap();
    assert!(!client.should_wait(&event("same-user")));
    assert_eq!(responses.lock().unwrap().len(), 1);
    assert_eq!(responses.lock().unwrap()[0].1, 0);
}

#[tokio::test]
async fn failed_batches_retry_separately_for_each_user_device_pair() {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            ResponseTemplate::new(if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                500
            } else {
                200
            })
        })
        .expect(3)
        .mount(&server)
        .await;
    let client = client(&server);
    client.log_event(event("first-device")).await.unwrap();
    let mut other = event("second-device");
    other.device_id = Some("another-device".into());
    client.log_event(other).await.unwrap();
    client.flush_events_and_wait().await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].body_json::<serde_json::Value>().unwrap()["events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for request in &requests[1..] {
        assert_eq!(
            request.body_json::<serde_json::Value>().unwrap()["events"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelling_track_does_not_cancel_accepted_delivery() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(50)))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    let (responses, callback) = record();
    client.set_callbacks(Some(callback));
    let sending = client.clone();
    let task = tokio::spawn(async move { sending.track(event("accepted")).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    task.abort();
    client.flush_events_and_wait().await.unwrap();
    assert_eq!(responses.lock().unwrap().len(), 1);
    assert_eq!(responses.lock().unwrap()[0].1, 200);
    client.shutdown().await.unwrap();
}
