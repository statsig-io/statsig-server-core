use amplitude_rust::{Amplitude, Config, Event, EventOptions, IdentifyOperation, ServerZone};
use serde_json::json;
use std::{
    env,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "sends live events to Amplitude using AMPLITUDE_API_KEY"]
async fn sends_live_identify_and_track() {
    let flush_interval = Duration::from_secs(3);
    let client = live_client(flush_interval);

    let run_id = format!("rust-{}", Uuid::new_v4());
    let user_id = format!("sdk-smoke-{run_id}");
    println!("Rust smoke run {run_id}, user ID {user_id}");
    let options = EventOptions {
        user_id: Some(user_id),
        ..EventOptions::default()
    };
    let properties = json!({"smoke_run_id": run_id, "sdk": "rust"})
        .as_object()
        .cloned();
    let track_event = |number| Event {
        options: options.clone(),
        event_properties: json!({"smoke_run_id": run_id, "sdk": "rust", "track_number": number})
            .as_object()
            .cloned(),
        ..Event::new("SDK Smoke Test")
    };

    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(2), async {
        let operations = vec![IdentifyOperation::Set {
            key: "sdk_smoke_run_id".into(),
            value: json!(run_id),
        }];
        assert_eq!(
            client
                .identify(operations, options.clone(), properties.clone())
                .await
                .unwrap(),
            0
        );
        for number in 1..=2 {
            assert_eq!(client.track(track_event(number)).await.unwrap(), 0);
        }
    })
    .await
    .expect("Queued calls should return before the flush interval");

    let delivery = tokio::time::timeout(
        Duration::from_secs(15),
        client.track_with_result(track_event(3)),
    )
    .await
    .expect("The flush timer should send the batch")
    .unwrap();
    assert_eq!(delivery.code, 200);
    assert!(started.elapsed() >= flush_interval);
    client.shutdown().await;
    println!("Amplitude accepted the Rust smoke batch after the three-second timer (HTTP 200)");
}

#[tokio::test]
#[ignore = "sends a live event to Amplitude using AMPLITUDE_API_KEY"]
async fn sends_live_event_on_shutdown() {
    let client = live_client(Duration::from_secs(10));
    let run_id = format!("rust-shutdown-{}", Uuid::new_v4());
    let user_id = format!("sdk-smoke-{run_id}");
    println!("Rust shutdown smoke run {run_id}, user ID {user_id}");
    let event = Event {
        options: EventOptions {
            user_id: Some(user_id),
            ..EventOptions::default()
        },
        event_properties: json!({"smoke_run_id": run_id, "sdk": "rust"})
            .as_object()
            .cloned(),
        ..Event::new("SDK Smoke Test Shutdown")
    };

    let results = tokio::time::timeout(Duration::from_secs(9), async {
        assert_eq!(client.track(event).await.unwrap(), 0);
        client.shutdown().await
    })
    .await
    .expect("Shutdown should send the event before the ten-second flush timer");
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].as_slice(), [Ok(200)]), "{results:?}");
    println!("Amplitude accepted the Rust shutdown event (HTTP 200)");
}

fn live_client(flush_interval: Duration) -> Amplitude {
    let api_key =
        env::var("AMPLITUDE_API_KEY").expect("Set AMPLITUDE_API_KEY for the live smoke test");
    let server_zone = match env::var("AMPLITUDE_SERVER_ZONE").as_deref() {
        Ok("eu") => ServerZone::Eu,
        Ok("us") | Err(env::VarError::NotPresent) => ServerZone::Us,
        _ => panic!("AMPLITUDE_SERVER_ZONE must be us or eu"),
    };
    Amplitude::new(
        &api_key,
        Some(Config {
            flush_interval_millis: Some(flush_interval.as_secs_f64() * 1_000.0),
            server_zone: Some(server_zone),
            ..Config::default()
        }),
    )
    .unwrap()
}
