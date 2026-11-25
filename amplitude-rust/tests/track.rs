use amplitude_rust::{Amplitude, AmplitudeError, BaseEvent, Config, EventOptions, GroupName};
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn sends_a_track_event_with_wit_fields() {
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
    let event = BaseEvent {
        options: EventOptions {
            user_id: Some("user-123".into()),
            time: Some(1234567890000),
            session_id: Some(1234567890000),
            insert_id: Some("event-123".into()),
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
        ..BaseEvent::new("Purchase Completed")
    };
    client.track(event).await.unwrap();
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
            ..Config::default()
        }),
    )
    .unwrap();

    assert!(matches!(
        client.track(BaseEvent::new("Purchase Completed")).await,
        Err(AmplitudeError::InvalidInput(_))
    ));
    let mut event = BaseEvent::new("");
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
