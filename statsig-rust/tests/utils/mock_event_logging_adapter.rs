use async_trait::async_trait;
use serde_json::Value;
use statsig_rust::log_event_payload::{LogEventPayload, LogEventRequest};
use statsig_rust::{EventLoggingAdapter, StatsigErr, StatsigRuntime};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MockEventLoggingAdapter {
    pub logged_event_count: AtomicU64,
    pub logged_payloads: Mutex<Vec<LogEventPayload>>,
}

impl Default for MockEventLoggingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventLoggingAdapter {
    pub fn new() -> Self {
        Self {
            logged_event_count: AtomicU64::new(0),
            logged_payloads: Mutex::new(Vec::new()),
        }
    }

    pub async fn force_get_first_event(&self) -> HashMap<String, Value> {
        let first_payload = self.force_get_received_payloads().await;
        let non_diagnostic_events = self.filter_non_diagnostic_events(&first_payload);
        let first_event = non_diagnostic_events
            .first()
            .expect("No non-diagnostic events found!");

        let event_obj = first_event.as_object().unwrap();

        event_obj
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub async fn force_get_received_payloads(&self) -> LogEventPayload {
        self.logged_payloads.lock().await.first().unwrap().clone()
    }

    fn filter_non_diagnostic_events(&self, payload: &LogEventPayload) -> Vec<Value> {
        let empty_vec = Vec::new();
        let events = payload.events.as_array().unwrap_or(&empty_vec);
        events
            .iter()
            .filter(|event| {
                if let Some(obj) = event.as_object() {
                    if let Some(event_name) = obj.get("eventName") {
                        return event_name != &Value::String("statsig::diagnostics".to_string());
                    }
                }
                false
            })
            .cloned()
            .collect()
    }
}

#[async_trait]
impl EventLoggingAdapter for MockEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let mut payloads = self.logged_payloads.lock().await;

        self.logged_event_count
            .fetch_add(request.event_count, Ordering::SeqCst);
        payloads.push(request.payload);

        Ok(true)
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        true
    }
}
