mod utils;

use serial_test::serial;
use statsig_rust::{
    output_logger::LogLevel, ObservabilityClient, OpsStatsEventObserver, Statsig, StatsigOptions,
    StatsigUser,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
};

use crate::utils::helpers::load_contents;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

const SDK_KEY: &str = "secret-key";

async fn setup(observability_client: &Arc<MockObservabilityClient>) -> (MockScrapi, Statsig) {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{\"success\": true}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let mut raw_dcs_str = load_contents("eval_proj_dcs.json");
    raw_dcs_str = raw_dcs_str.replace(
        r#""checksum":"8506699639233708000""#,
        r#""IGNORED_CHECKSUM_VALUE":"""#,
    );

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(raw_dcs_str),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let weak_obs_client = Arc::downgrade(observability_client) as Weak<dyn ObservabilityClient>;
    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            observability_client: Some(weak_obs_client),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            output_log_level: Some(LogLevel::Debug),
            specs_sync_interval_ms: Some(1),
            ..StatsigOptions::new()
        })),
    );

    (mock_scrapi, statsig)
}

#[derive(Debug, PartialEq)]
enum RecordedCall {
    Init,
    Increment(
        String,
        f64,
        Option<std::collections::HashMap<String, String>>,
    ),
    Gauge(
        String,
        f64,
        Option<std::collections::HashMap<String, String>>,
    ),
    Dist(
        String,
        f64,
        Option<std::collections::HashMap<String, String>>,
    ),
    Error(String, String),
}

struct MockObservabilityClient {
    pub calls: Mutex<Vec<RecordedCall>>,
}

impl ObservabilityClient for MockObservabilityClient {
    fn init(&self) {
        self.calls.lock().unwrap().push(RecordedCall::Init);
    }

    fn increment(
        &self,
        metric_name: String,
        value: f64,
        tags: Option<std::collections::HashMap<String, String>>,
    ) {
        self.calls
            .lock()
            .unwrap()
            .push(RecordedCall::Increment(metric_name, value, tags));
    }

    fn gauge(
        &self,
        metric_name: String,
        value: f64,
        tags: Option<std::collections::HashMap<String, String>>,
    ) {
        self.calls
            .lock()
            .unwrap()
            .push(RecordedCall::Gauge(metric_name, value, tags));
    }

    fn dist(
        &self,
        metric_name: String,
        value: f64,
        tags: Option<std::collections::HashMap<String, String>>,
    ) {
        self.calls
            .lock()
            .unwrap()
            .push(RecordedCall::Dist(metric_name, value, tags));
    }

    fn error(&self, tag: String, error: String) {
        self.calls
            .lock()
            .unwrap()
            .push(RecordedCall::Error(tag, error));
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }

    fn should_enable_high_cardinality_for_this_tag(&self, _tag: String) -> Option<bool> {
        Some(true)
    }
}

#[tokio::test]
#[serial]
async fn test_init_called() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig) = setup(&obs_client).await;

    statsig.initialize().await.unwrap();

    let calls = obs_client.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], RecordedCall::Init);
}

#[tokio::test]
#[serial]
async fn test_sdk_initialization_dist_recorded() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig) = setup(&obs_client).await;

    statsig.initialize().await.unwrap();
    statsig.check_gate(&StatsigUser::with_user_id("test_user"), "test_gate");
    statsig.flush_events().await;

    let calls = obs_client.calls.lock().unwrap();

    let mut found_name = String::new();
    let mut found_value = 0.0;
    let mut found_tags = None;

    for call in calls.iter() {
        if let RecordedCall::Dist(metric_name, value, tags) = call {
            if metric_name == "statsig.sdk.initialization" {
                found_name = metric_name.clone();
                found_value = *value;
                found_tags = tags.clone();
                break;
            }
        }
    }

    let tags = found_tags.unwrap();

    assert_eq!(found_name, "statsig.sdk.initialization");
    assert_ne!(found_value, 0.0);
    assert_eq!(tags.get("source"), Some(&"Network".to_string()));
    assert_eq!(tags.get("init_success"), Some(&"true".to_string()));
    assert_eq!(tags.get("store_populated"), Some(&"true".to_string()));
}

#[tokio::test]
#[serial]
async fn test_config_propagation_dist_recorded() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig) = setup(&obs_client).await;

    statsig.initialize().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let calls = obs_client.calls.lock().unwrap();

    let mut found_name = String::new();
    let mut found_value = 0.0;
    let mut found_tags = None;

    for call in calls.iter() {
        if let RecordedCall::Dist(metric_name, value, tags) = call {
            if metric_name == "statsig.sdk.config_propagation_diff" {
                found_name = metric_name.clone();
                found_value = *value;
                found_tags = tags.clone();
                break;
            }
        }
    }

    let tags = found_tags.unwrap();

    assert_eq!(found_name, "statsig.sdk.config_propagation_diff");
    assert_ne!(found_value, 0.0);
    assert_eq!(tags.get("source"), Some(&"Network".to_string()));
    assert!(tags.contains_key("lcut"));
}

#[tokio::test]
#[serial]
async fn test_error_callback_called() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let weak_obs_client = Arc::downgrade(&obs_client) as Weak<dyn ObservabilityClient>;
    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            observability_client: Some(weak_obs_client),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        })),
    );

    let _ = statsig.initialize().await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let calls = obs_client.calls.lock().unwrap();
    assert!(calls.len() >= 3); // one init, one sdk initialization, and at least one error callback
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, RecordedCall::Error(_, _))),
        "Expected at least one RecordedCall::Error, but found none"
    );
}

#[tokio::test]
#[serial]
async fn test_shutdown_drops() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig) = setup(&obs_client).await;

    statsig.initialize().await.unwrap();

    assert_eq!(Arc::strong_count(&obs_client), 1);

    statsig.flush_events().await;
    statsig.shutdown().await.unwrap();
    drop(statsig);

    assert_eq!(Arc::strong_count(&obs_client), 1);
}

#[tokio::test]
#[serial]
async fn test_init_from_network() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let mock_scrapi = MockScrapi::new().await;

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/eval_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read file");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(dcs),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{\"success\": true}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let weak_obs_client = Arc::downgrade(&obs_client) as Weak<dyn ObservabilityClient>;
    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            observability_client: Some(weak_obs_client),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            output_log_level: Some(LogLevel::Debug),
            specs_sync_interval_ms: Some(1),
            ..StatsigOptions::new()
        })),
    );

    let _ = statsig.initialize().await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let calls = obs_client.calls.lock().unwrap();

    let mut found_name = String::new();
    let mut found_value = 0.0;
    let mut found_tags = None;

    for call in calls.iter() {
        if let RecordedCall::Dist(metric_name, value, tags) = call {
            if metric_name == "statsig.sdk.initialization" {
                found_name = metric_name.clone();
                found_value = *value;
                found_tags = tags.clone();
                break;
            }
        }
    }

    let tags = found_tags.unwrap();

    assert_eq!(found_name, "statsig.sdk.initialization");
    assert_ne!(found_value, 0.0);
    assert_eq!(tags.get("source"), Some(&"Network".to_string()));
    assert_eq!(tags.get("init_success"), Some(&"true".to_string()));
    assert_eq!(tags.get("store_populated"), Some(&"true".to_string()));
    assert_eq!(
        tags.get("init_source_api"),
        Some(&mock_scrapi.get_server_api())
    );
}
