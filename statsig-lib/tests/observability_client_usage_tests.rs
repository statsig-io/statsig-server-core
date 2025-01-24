mod utils;

use sigstat::{output_logger::LogLevel, ObservabilityClient, Statsig, StatsigOptions, StatsigUser};
use std::sync::{Arc, Mutex};
use utils::{
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
    mock_specs_adapter::MockSpecsAdapter,
};

const SDK_KEY: &str = "secret-key";

async fn setup(
    observability_client: Option<Arc<dyn ObservabilityClient>>,
) -> (MockScrapi, Statsig, Arc<MockSpecsAdapter>) {
    let mock_scrapi = MockScrapi::new().await;

    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            observability_client,
            specs_adapter: Some(specs_adapter.clone()),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            output_log_level: Some(LogLevel::Debug),
            specs_sync_interval_ms: Some(1),
            ..StatsigOptions::new()
        })),
    );

    (mock_scrapi, statsig, specs_adapter)
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

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn sigstat::OpsStatsEventObserver> {
        self
    }
}

#[tokio::test]
async fn test_init_called() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig, _) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();

    let calls = obs_client.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], RecordedCall::Init);
}

#[tokio::test]
async fn test_sdk_initialization_dist_recorded() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig, _) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();
    statsig.check_gate(&StatsigUser::with_user_id("test_user".into()), "test_gate");
    statsig.flush_events().await;

    let calls = obs_client.calls.lock().unwrap();

    let mut found_name = "".to_string();
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
    assert_eq!(tags.get("source"), Some(&"Bootstrap".to_string()));
    assert_eq!(tags.get("success"), Some(&"true".to_string()));
    assert_eq!(tags.get("store_populated"), Some(&"true".to_string()));
}

#[tokio::test]
async fn test_config_propagation_dist_recorded() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig, specs_adapter) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();
    specs_adapter.resync().await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let calls = obs_client.calls.lock().unwrap();

    let mut found_name = "".to_string();
    let mut found_value = 0.0;
    let mut found_tags = None;

    for call in calls.iter() {
        if let RecordedCall::Dist(metric_name, value, tags) = call {
            if metric_name == "statsig.sdk.config_propogation_diff" {
                found_name = metric_name.clone();
                found_value = *value;
                found_tags = tags.clone();
                break;
            }
        }
    }

    let tags = found_tags.unwrap();

    assert_eq!(found_name, "statsig.sdk.config_propogation_diff");
    assert_ne!(found_value, 0.0);
    assert_eq!(tags.get("source"), Some(&"Bootstrap".to_string()));
}

#[tokio::test]
async fn test_shutdown_drops() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig, _) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();

    assert_eq!(Arc::strong_count(&obs_client), 2);

    statsig.flush_events().await;
    statsig.shutdown().await.unwrap();
    drop(statsig);

    assert_eq!(Arc::strong_count(&obs_client), 1);
}
