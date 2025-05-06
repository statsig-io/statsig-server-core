use statsig_rust::{ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type MetricCalls = Vec<(String, f64, Option<HashMap<String, String>>)>;

pub struct MockObservabilityClient {
    pub init_called_times: AtomicU64,
    pub increment_calls: Mutex<MetricCalls>,
    pub gauge_calls: Mutex<MetricCalls>,
    pub dist_calls: Mutex<MetricCalls>,
    pub error_calls: Mutex<Vec<(String, String)>>,
}

impl MockObservabilityClient {
    pub fn new() -> Self {
        Self {
            init_called_times: AtomicU64::new(0),
            increment_calls: Mutex::new(Vec::new()),
            gauge_calls: Mutex::new(Vec::new()),
            dist_calls: Mutex::new(Vec::new()),
            error_calls: Mutex::new(Vec::new()),
        }
    }
}

impl ObservabilityClient for MockObservabilityClient {
    fn init(&self) {
        let _ = self.init_called_times.fetch_add(1, Ordering::SeqCst);
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.increment_calls
            .lock()
            .unwrap()
            .push((metric_name, value, tags));
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.gauge_calls
            .lock()
            .unwrap()
            .push((metric_name, value, tags));
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        self.dist_calls
            .lock()
            .unwrap()
            .push((metric_name, value, tags));
    }

    fn error(&self, tag: String, error: String) {
        self.error_calls.lock().unwrap().push((tag, error));
    }

    fn should_enable_high_cardinality_for_this_tag(&self, _tag: String) -> Option<bool> {
        Some(false)
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}
