use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::observability::ops_stats::OpsStatsEventObserver;

use super::ops_stats::OpsStatsEvent;

static HIGH_CARDINALITY_TAGS: &[&str] = &["lcut", "prev_lcut"];

#[derive(Clone)]
pub enum MetricType {
    Increment,
    Gauge,
    Dist,
}

#[derive(Clone)]
pub struct ObservabilityEvent {
    pub metric_type: MetricType,
    pub metric_name: String,
    pub value: f64,
    pub tags: Option<HashMap<String, String>>,
}

impl ObservabilityEvent {
    pub fn new_event(
        metric_type: MetricType,
        metric_name: String,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) -> OpsStatsEvent {
        OpsStatsEvent::Observability(ObservabilityEvent {
            metric_type,
            metric_name,
            value,
            tags,
        })
    }
}

pub trait ObservabilityClient: Send + Sync + 'static + OpsStatsEventObserver {
    fn init(&self);
    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn error(&self, tag: String, error: String);
    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool>;
    // This is needed since upper casting is not officially supported yet
    // (WIP to support https://github.com/rust-lang/rust/issues/65991)
    // For implementation, just return Self; should be sufficient
    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver>;
}

#[async_trait]
impl<T: ObservabilityClient> OpsStatsEventObserver for T {
    async fn handle_event(&self, event: OpsStatsEvent) {
        match event {
            OpsStatsEvent::Observability(data) => {
                let tags: Option<HashMap<String, String>> = data.tags.map(|tags_unwrapped| {
                    tags_unwrapped
                        .into_iter()
                        .filter(|(key, _)| {
                            if HIGH_CARDINALITY_TAGS.contains(&key.as_str()) {
                                self.should_enable_high_cardinality_for_this_tag(key.to_string())
                                    .unwrap_or_default()
                            } else {
                                true
                            }
                        })
                        .collect()
                });
                let metric_name = format!("statsig.sdk.{}", data.metric_name.clone());
                match data.metric_type {
                    MetricType::Increment => self.increment(metric_name, data.value, tags),
                    MetricType::Gauge => self.gauge(metric_name, data.value, tags),
                    MetricType::Dist => self.dist(metric_name, data.value, tags),
                };
            }
            OpsStatsEvent::SDKError(error) => {
                self.error(error.tag, error.info.to_string());
            }
            _ => {}
        }
    }
}
