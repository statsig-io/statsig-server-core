use crate::statsig_options::StatsigOptions;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait EventLoggingAdapter: Send + Sync {
    fn bind(&self, sdk_key: &str, option: &StatsigOptions);
    async fn log_events(
        &self,
        payload: HashMap<String, Value>,
        event_count: usize,
    ) -> Result<bool, String>;
}
