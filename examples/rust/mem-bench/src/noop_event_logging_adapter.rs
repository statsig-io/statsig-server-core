use async_trait::async_trait;
use statsig_rust::log_event_payload::LogEventRequest;
use statsig_rust::{EventLoggingAdapter, StatsigErr, StatsigRuntime};
use std::sync::Arc;

#[derive(Default)]
pub struct NoopEventLoggingAdapter;

#[async_trait]
impl EventLoggingAdapter for NoopEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, _request: LogEventRequest) -> Result<bool, StatsigErr> {
        Ok(true)
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        true
    }
}
