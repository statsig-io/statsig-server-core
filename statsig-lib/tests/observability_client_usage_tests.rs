mod utils;

use sigstat::{output_logger::LogLevel, ObservabilityClient, Statsig, StatsigOptions};
use std::sync::{Arc, Mutex};
use utils::{
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
    mock_specs_adapter::MockSpecsAdapter,
};

const SDK_KEY: &str = "secret-key";

async fn setup(
    observability_client: Option<Arc<dyn ObservabilityClient>>,
) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

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
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
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

    let (_, statsig) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();

    let calls = obs_client.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], RecordedCall::Init);
}

#[tokio::test]
async fn test_shutdown_drops() {
    let obs_client = Arc::new(MockObservabilityClient {
        calls: Mutex::new(Vec::new()),
    });

    let (_, statsig) = setup(Some(obs_client.clone())).await;

    statsig.initialize().await.unwrap();

    assert_eq!(Arc::strong_count(&obs_client), 2);

    statsig.flush_events().await;
    statsig.shutdown().await.unwrap();
    drop(statsig);

    assert_eq!(Arc::strong_count(&obs_client), 1);
}
