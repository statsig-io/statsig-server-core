use amplitude_rust::{
    Amplitude, AmplitudeError, Config, Event, EventOptions, GroupName, IdentifyOperation,
};
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn sends_a_track_event_with_event_options() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/httpapi"))
        .and(body_json(json!({
            "api_key": "test-api-key",
            "events": [{
                "event_type": "Purchase Completed",
                "user_id": "user-123",
                "event_properties": {"total": 29.99, "items": ["shirt"]},
                "user_properties": {"plan": "free"},
                "groups": {"company": "example", "team": ["north", "south"]},
                "group_properties": {"tier": "enterprise"},
                "time": 1234567890000_i64,
                "session_id": 1234567890000_i64,
                "insert_id": "event-123",
                "productId": "shirt",
                "revenueType": "purchase",
                "library": "custom-sdk/1.0",
            }],
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new(
        "test-api-key",
        Some(Config {
            server_url: Some(format!("{}/2/httpapi", server.uri())),
            ..Config::default()
        }),
    )
    .unwrap();
    let event = Event {
        options: EventOptions {
            user_id: Some("user-123".into()),
            time: Some(1234567890000),
            session_id: Some(1234567890000),
            insert_id: Some("event-123".into()),
            product_id: Some("shirt".into()),
            revenue_type: Some("purchase".into()),
            ..EventOptions::default()
        },
        event_properties: json!({"total": 29.99, "items": ["shirt"]})
            .as_object()
            .cloned(),
        user_properties: json!({"plan": "free"}).as_object().cloned(),
        groups: Some(
            [
                ("company".into(), GroupName::Single("example".into())),
                (
                    "team".into(),
                    GroupName::Multiple(vec!["north".into(), "south".into()]),
                ),
            ]
            .into(),
        ),
        group_properties: json!({"tier": "enterprise"}).as_object().cloned(),
        library: Some("custom-sdk/1.0".into()),
        ..Event::new("Purchase Completed")
    };
    let sdk_payload = serde_json::to_value(&event).unwrap();
    assert_eq!(sdk_payload["options"]["product_id"], "shirt");
    assert_eq!(sdk_payload["options"]["revenue_type"], "purchase");
    assert_eq!(client.track(event).await.unwrap(), 200);
}

#[tokio::test]
async fn validates_track_requests_and_reports_http_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/httpapi"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new(
        "test-api-key",
        Some(Config {
            server_url: Some(format!("{}/2/httpapi", server.uri())),
            flush_max_retries: Some(2.0),
            ..Config::default()
        }),
    )
    .unwrap();

    assert!(matches!(
        client.track(Event::new("Purchase Completed")).await,
        Err(AmplitudeError::InvalidInput(_))
    ));
    let mut event = Event::new("");
    event.options.device_id = Some("device-123".into());
    assert!(matches!(
        client.track(event.clone()).await,
        Err(AmplitudeError::InvalidInput(_))
    ));
    event.event_type = "Purchase Completed".into();
    assert!(matches!(
        client.track(event).await,
        Err(AmplitudeError::Api { status: 400 })
    ));

    let requests = server.received_requests().await.unwrap();
    let payload: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let event = &payload["events"][0];
    assert_eq!(event["device_id"], "device-123");
    assert!(event.get("user_id").is_none());
    assert!(event["time"].as_i64().unwrap() > 0);
    assert!(Uuid::parse_str(event["insert_id"].as_str().unwrap()).is_ok());
    assert_eq!(
        event["library"],
        concat!("amplitude-rust-core/", env!("CARGO_PKG_VERSION"))
    );
}

#[tokio::test]
async fn flushes_track_and_identify_together_and_reuses_payload_on_retry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .with_priority(1)
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new(
        "test-api-key",
        Some(Config {
            server_url: Some(server.uri()),
            flush_interval_millis: Some(60_000.0),
            flush_queue_size: Some(2.0),
            flush_max_retries: Some(1.0),
            ..Config::default()
        }),
    )
    .unwrap();
    let options = EventOptions {
        user_id: Some("user-123".into()),
        ..EventOptions::default()
    };
    let event = Event {
        options: options.clone(),
        ..Event::new("Purchase Completed")
    };
    let (track, identify) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            client.track(event),
            client.identify(vec![IdentifyOperation::ClearAll], options, None),
        )
    })
    .await
    .unwrap();
    assert_eq!(track.unwrap(), 200);
    assert_eq!(identify.unwrap(), 200);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    let payload: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let events = payload["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "Purchase Completed");
    assert_eq!(events[1]["event_type"], "$identify");
    assert_eq!(events[1]["user_properties"], json!({"$clearAll": "-"}));
    assert_ne!(events[0]["insert_id"], events[1]["insert_id"]);
}

#[tokio::test]
async fn flushes_a_partial_queue_at_the_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new(
        "test-api-key",
        Some(Config {
            server_url: Some(server.uri()),
            flush_interval_millis: Some(20.0),
            flush_queue_size: Some(2.0),
            ..Config::default()
        }),
    )
    .unwrap();
    let mut event = Event::new("App Opened");
    event.options.device_id = Some("device-123".into());
    let start = std::time::Instant::now();
    let status = tokio::time::timeout(std::time::Duration::from_secs(5), client.track(event))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status, 200);
    assert!(start.elapsed() >= std::time::Duration::from_millis(20));
}
