use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::Value;

use crate::observability::ops_stats::{OpsStatsEvent, OpsStatsForInstance, OPS_STATS};
use crate::{OpsStatsEventObserver, StatsigRuntime};

/// Records every diagnostics marker added for one SDK key, as
/// (context, serialized marker).
pub struct RecordingDiagnosticsObserver {
    markers: Mutex<Vec<(String, Value)>>,
    // OPS_STATS only holds a Weak per SDK key, and dropping a StatsigRuntime
    // stops its tasks. Keep both alive so the subscription keeps receiving.
    subscription: Mutex<Option<(Arc<OpsStatsForInstance>, Arc<StatsigRuntime>)>>,
}

impl RecordingDiagnosticsObserver {
    /// Subscribe before creating the code under test so no marker is missed.
    /// Keep the returned Arc alive for as long as markers should be recorded.
    pub fn subscribe(sdk_key: &str) -> Arc<Self> {
        let observer = Arc::new(Self {
            markers: Mutex::new(Vec::new()),
            subscription: Mutex::new(None),
        });
        let ops_stats = OPS_STATS.get_for_instance(sdk_key);
        let runtime = StatsigRuntime::get_runtime();
        let weak: std::sync::Weak<dyn OpsStatsEventObserver> = Arc::downgrade(&observer) as _;
        ops_stats.subscribe(runtime.clone(), weak);
        *observer.subscription.lock() = Some((ops_stats, runtime));
        observer
    }

    pub fn markers(&self) -> Vec<(String, Value)> {
        self.markers.lock().clone()
    }

    /// Waits until at least `count` markers have arrived; events are delivered asynchronously.
    pub async fn wait_for_markers(&self, count: usize) -> Vec<(String, Value)> {
        for _ in 0..500 {
            if self.markers.lock().len() >= count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.markers()
    }
}

#[async_trait]
impl OpsStatsEventObserver for RecordingDiagnosticsObserver {
    async fn handle_event(&self, event: OpsStatsEvent) {
        let OpsStatsEvent::Diagnostics(event) = event else {
            return;
        };
        let Some(marker) = event.marker else {
            return;
        };
        let context = event.context.to_string();
        let marker = serde_json::to_value(&marker).unwrap_or_default();
        self.markers.lock().push((context, marker));
    }
}

/// "key:step:action" for a serialized marker.
pub fn summarize_marker(marker: &Value) -> String {
    let mut parts = vec![marker["key"].as_str().unwrap_or_default()];
    if let Some(step) = marker["step"].as_str() {
        parts.push(step);
    }
    parts.push(marker["action"].as_str().unwrap_or_default());
    parts.join(":")
}
