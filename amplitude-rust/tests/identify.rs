use amplitude_rust::{
    Amplitude, AmplitudeError, Config, EventOptions, IdentifyOperation, IngestionMetadata, Plan,
};
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn sends_an_identify_event_with_all_property_operations() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/httpapi"))
        .and(body_json(json!({
            "api_key": "test-api-key",
            "events": [{
                "event_type": "$identify",
                "user_id": "abc",
                "event_properties": {"source": "account"},
                "user_properties": {
                    "$set": {"plan": "pro", "verified": true},
                    "$setOnce": {"signup": "email"},
                    "$append": {"tags": ["ruby", "rust"]},
                    "$prepend": {"history": "latest"},
                    "$preInsert": {"segments": "early"},
                    "$postInsert": {"cohorts": "beta"},
                    "$remove": {"flags": "legacy"},
                    "$add": {"logins": 2.5},
                    "$unset": {"old_plan": "-"}
                },
                "time": 1234567890000_i64,
                "session_id": 1234567890000_i64,
                "insert_id": "event-123",
                "app_version": "1.2",
                "plan": {"branch": "main"},
                "ingestion_metadata": {"source_name": "account"},
                "library": concat!("amplitude-rust-core/", env!("CARGO_PKG_VERSION")),
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
            plan: Some(Plan {
                branch: Some("main".into()),
                ..Plan::default()
            }),
            ..Config::default()
        }),
    )
    .unwrap();
    use IdentifyOperation::*;
    let operations = vec![
        Set {
            key: "plan".into(),
            value: json!("pro"),
        },
        Set {
            key: "verified".into(),
            value: json!(true),
        },
        SetOnce {
            key: "signup".into(),
            value: json!("email"),
        },
        Append {
            key: "tags".into(),
            value: json!(["ruby", "rust"]),
        },
        Prepend {
            key: "history".into(),
            value: json!("latest"),
        },
        PreInsert {
            key: "segments".into(),
            value: json!("early"),
        },
        PostInsert {
            key: "cohorts".into(),
            value: json!("beta"),
        },
        Remove {
            key: "flags".into(),
            value: json!("legacy"),
        },
        Add {
            key: "logins".into(),
            value: 2.5,
        },
        Unset {
            key: "old_plan".into(),
        },
    ];
    let options = EventOptions {
        user_id: Some("abc".into()),
        time: Some(1234567890000),
        session_id: Some(1234567890000),
        insert_id: Some("event-123".into()),
        app_version: Some("1.2".into()),
        ingestion_metadata: Some(IngestionMetadata {
            source_name: Some("account".into()),
            ..IngestionMetadata::default()
        }),
        ..EventOptions::default()
    };
    let event_properties = json!({"source": "account"}).as_object().cloned();
    client
        .identify(operations, options, event_properties)
        .await
        .unwrap();
}

#[tokio::test]
async fn clear_all_supports_device_only_events_and_surfaces_http_errors() {
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
    let options = EventOptions {
        device_id: Some("device-123".into()),
        ..EventOptions::default()
    };

    assert!(matches!(
        client
            .identify(vec![IdentifyOperation::ClearAll], options, None)
            .await,
        Err(AmplitudeError::Api { status: 400 })
    ));

    let requests = server.received_requests().await.unwrap();
    let payload: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let event = &payload["events"][0];
    assert_eq!(event["user_properties"], json!({"$clearAll": "-"}));
    assert_eq!(event["device_id"], "device-123");
    assert!(event.get("user_id").is_none());
    assert!(event["time"].as_i64().unwrap() > 0);
    assert!(Uuid::parse_str(event["insert_id"].as_str().unwrap()).is_ok());
}

#[tokio::test]
async fn rejects_invalid_identify_requests_before_sending() {
    let server = MockServer::start().await;
    let client = Amplitude::new(
        "test-api-key",
        Some(Config {
            server_url: Some(server.uri()),
            ..Config::default()
        }),
    )
    .unwrap();
    let options = EventOptions {
        user_id: Some("user-123".into()),
        ..EventOptions::default()
    };
    let clear = IdentifyOperation::ClearAll;
    let unset = IdentifyOperation::Unset { key: "plan".into() };

    assert!(matches!(
        client
            .identify(vec![clear.clone()], EventOptions::default(), None)
            .await,
        Err(AmplitudeError::InvalidInput(_))
    ));
    for operations in [
        vec![],
        vec![clear, unset.clone()],
        vec![unset.clone(), unset],
    ] {
        assert!(matches!(
            client.identify(operations, options.clone(), None).await,
            Err(AmplitudeError::InvalidInput(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
