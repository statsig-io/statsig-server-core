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

    let mut amplitude = Amplitude::new();
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
    let mut amplitude = Amplitude::new();
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
