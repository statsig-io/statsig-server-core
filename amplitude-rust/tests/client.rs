use amplitude_rust::{Amplitude, Error, Event, Options};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn options(server: &MockServer) -> Options {
    Options {
        server_url: Some(format!("{}/2/httpapi", server.uri())),
        retry_delay: Duration::from_millis(1),
        ..Options::default()
    }
}

fn event(name: &str) -> Event {
    let mut event = Event::new(name);
    event.user_id = Some("user-123".into());
    event
}

#[tokio::test]
async fn defaults_middleware_and_both_callbacks_reach_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/httpapi"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let calls = Arc::new(AtomicUsize::new(0));
    let client_calls = calls.clone();
    let mut options = options(&server);
    options.plan = Some(json!({"source":"server"}));
    options.ingestion_metadata = Some(json!({"source_name":"statsig"}));
    options.request_options = Some(json!({"min_id_length":1}));
    options.callback = Some(Arc::new(move |_, result| {
        assert!(result.is_ok());
        client_calls.fetch_add(1, Ordering::SeqCst);
    }));
    options.middleware.push(Arc::new(|event, extra| {
        event.event_properties.insert("extra".into(), extra.clone());
        event.event_type != "drop"
    }));
    let client = Amplitude::new("key", options).unwrap();
    let event_calls = calls.clone();
    client
        .log_event_with(
            event("keep"),
            Some(Arc::new(move |_, result| {
                assert!(result.is_ok());
                event_calls.fetch_add(1, Ordering::SeqCst);
            })),
            json!({"request":"123"}),
        )
        .await
        .unwrap();
    client.log_event(event("drop")).await.unwrap();
    client.flush_events().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_eq!(body["api_key"], "key");
    assert_eq!(body["events"].as_array().unwrap().len(), 1);
    assert_eq!(body["events"][0]["plan"], json!({"source":"server"}));
    assert_eq!(
        body["events"][0]["event_properties"]["extra"]["request"],
        "123"
    );
    assert_eq!(body["options"]["min_id_length"], 1);
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn retries_keep_insert_ids_and_report_terminal_failures_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .expect(3)
        .mount(&server)
        .await;
    let count = Arc::new(AtomicUsize::new(0));
    let callbacks = count.clone();
    let mut options = options(&server);
    options.max_retries = 2;
    options.callback = Some(Arc::new(move |_, result| {
        assert_eq!(*result, Err(Error::Http(503)));
        callbacks.fetch_add(1, Ordering::SeqCst);
    }));
    let client = Amplitude::new("key", options).unwrap();
    assert_eq!(
        client.send_events(vec![event("retry")]).await,
        Err(Error::Http(503))
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let requests = server.received_requests().await.unwrap();
    assert!(requests.windows(2).all(|pair| pair[0].body == pair[1].body));
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn threshold_and_period_flush_without_manual_flush() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&server)
        .await;
    let delivered = Arc::new(AtomicUsize::new(0));
    let count = delivered.clone();
    let mut options = options(&server);
    options.upload_threshold = 2;
    options.upload_period = Duration::from_millis(20);
    options.callback = Some(Arc::new(move |_, _| {
        count.fetch_add(1, Ordering::SeqCst);
    }));
    let client = Amplitude::new("key", options).unwrap();
    client.log_event(event("one")).await.unwrap();
    client.log_event(event("two")).await.unwrap();
    // Flush acts as an ordering barrier after the threshold-triggered upload.
    client.flush_events().await.unwrap();
    assert_eq!(delivered.load(Ordering::SeqCst), 2);
    client.log_event(event("three")).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while delivered.load(Ordering::SeqCst) < 3 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_drains_and_rejects_further_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new("key", options(&server)).unwrap();
    client.log_event(event("pending")).await.unwrap();
    client.shutdown().await.unwrap();
    client.shutdown().await.unwrap();
    assert_eq!(client.log_event(event("late")).await, Err(Error::Shutdown));
}

#[tokio::test]
async fn configuration_is_atomic_and_event_overrides_win() {
    let server = MockServer::start().await;
    Mock::given(path("/batch"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = Amplitude::new("key", options(&server)).unwrap();
    assert!(client
        .configure(|options| options.upload_threshold = 0)
        .await
        .is_err());
    let batch_url = format!("{}/batch", server.uri());
    client
        .configure(move |options| {
            options.server_url = Some(batch_url);
            options.batch_mode = true;
            options.plan = Some(json!({"source":"default"}));
        })
        .await
        .unwrap();
    let mut event = event("override");
    event.plan = Some(json!({"source":"event"}));
    client.send_events(vec![event]).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_eq!(body["events"][0]["plan"]["source"], "event");
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn payload_too_large_is_split_and_bad_requests_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let body: serde_json::Value = request.body_json().unwrap();
            ResponseTemplate::new(if body["events"].as_array().unwrap().len() > 1 {
                413
            } else {
                400
            })
        })
        .expect(3)
        .mount(&server)
        .await;
    let client = Amplitude::new("key", options(&server)).unwrap();
    assert_eq!(
        client.send_events(vec![event("one"), event("two")]).await,
        Err(Error::Http(400))
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn named_instances_are_shared_and_replaced_after_shutdown() {
    let server = MockServer::start().await;
    let a = Amplitude::get_instance("test-named", "key", options(&server)).unwrap();
    let b = Amplitude::get_instance("test-named", "ignored", options(&server)).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    a.shutdown().await.unwrap();
    let c = Amplitude::get_instance("test-named", "key", options(&server)).unwrap();
    assert!(!Arc::ptr_eq(&a, &c));
    c.shutdown().await.unwrap();
}

#[tokio::test]
async fn flush_timeout_bounds_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&server)
        .await;
    let mut options = options(&server);
    options.retry_delay = Duration::from_secs(5);
    options.flush_timeout = Duration::from_millis(50);
    let client = Amplitude::new("key", options).unwrap();
    assert_eq!(
        client.send_events(vec![event("timeout")]).await,
        Err(Error::Timeout)
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn throttled_ids_are_visible_only_during_active_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429))
        .expect(2)
        .mount(&server)
        .await;
    let mut options = options(&server);
    options.record_throttled_ids = true;
    options.retry_delay = Duration::from_millis(100);
    options.max_retries = 1;
    let client = Amplitude::new("key", options).unwrap();
    let sending = client.clone();
    let task = tokio::spawn(async move { sending.send_events(vec![event("throttled")]).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !client.should_wait(&event("same-user")) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let mut other = event("other-user");
    other.user_id = Some("different-user".into());
    assert!(!client.should_wait(&other));
    assert_eq!(task.await.unwrap(), Err(Error::Http(429)));
    assert!(!client.should_wait(&event("same-user")));
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn canceling_a_delivery_waiter_does_not_cancel_accepted_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(50)))
        .expect(1)
        .mount(&server)
        .await;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let mut options = options(&server);
    options.callback = Some(Arc::new(move |_, result| {
        assert!(result.is_ok());
        count.fetch_add(1, Ordering::SeqCst);
    }));
    let client = Amplitude::new("key", options).unwrap();
    let sending = client.clone();
    let waiter = tokio::spawn(async move { sending.send_events(vec![event("accepted")]).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    waiter.abort();
    client.shutdown().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
