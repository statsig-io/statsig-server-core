use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use async_trait::async_trait;
use serde_json::{Map, Value};
use tokio::sync::Mutex;
use sigstat::{EventLoggingAdapter, StatsigOptions};

pub struct MockEventLoggingAdapter {
    pub logged_event_count: AtomicUsize,
    pub logged_payloads: Mutex<Vec<HashMap<String, Value>>>
}

impl Default for MockEventLoggingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventLoggingAdapter {
    pub fn new() -> Self {
        Self {
            logged_event_count: AtomicUsize::new(0),
            logged_payloads: Mutex::new(Vec::new())
        }
    }

    pub async fn force_get_first_event(&self) -> Map<String, Value> {
        let first_payload = self.force_get_received_payloads().await;
        let events = first_payload.get("events").unwrap();
        let event = events.get(0).cloned().unwrap().as_object().cloned().unwrap();
        event
    }

    pub async fn force_get_received_payloads(&self) -> HashMap<String, Value> {
        self.logged_payloads.lock().await.first().unwrap().clone()
    }
}

#[async_trait]
impl EventLoggingAdapter for MockEventLoggingAdapter {
    fn bind(&self, _sdk_key: &str, _option: &StatsigOptions) {}

    async fn log_events(
        &self,
        payload: HashMap<String, Value>,
        event_count: usize
    ) -> Result<bool, String> {
        let mut payloads = self.logged_payloads.lock().await;

        self.logged_event_count.fetch_add(event_count, Ordering::SeqCst);
        payloads.push(payload.clone());

        Ok(true)
    }
}
