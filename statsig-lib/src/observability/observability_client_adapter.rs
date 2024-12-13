use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::observability::ops_stats::OpsStatsEventObserver;

use super::ops_stats::OpsStatsEvent;
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
        OpsStatsEvent::ObservabilityEvent(ObservabilityEvent {
            metric_type,
            metric_name, 
            value, 
            tags
        })
    }
}
pub trait ObservabilityClient: Send + Sync + 'static + OpsStatsEventObserver {
    fn init(&self);
    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>);
    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver>;
}

#[async_trait]
impl<T: ObservabilityClient> OpsStatsEventObserver for T {
    async fn handle_event(&self, event: OpsStatsEvent) {
        match event {
            OpsStatsEvent::ObservabilityEvent(data) => {
                let metric_name = format!("statsig.{}", data.clone().metric_name);
                match data.clone().metric_type {
                    MetricType::Increment => {
                        self.increment(metric_name, data.value, data.tags)
                    }
                    MetricType::Gauge => self.gauge(metric_name, data.value, data.tags),
                    MetricType::Dist => self.dist(metric_name, data.value, data.tags),
                };
            }
        }
    }
}