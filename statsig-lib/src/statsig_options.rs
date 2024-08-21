use crate::SpecsAdapter;
use std::sync::Arc;
use crate::event_logging_adapter::event_logging_adapter::EventLoggingAdapter;

#[derive(Clone, Default)]
pub struct StatsigOptions {
    pub specs_url: Option<String>,
    pub log_event_url: Option<String>,
    pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
    pub specs_sync_interval_ms: Option<u32>,
    pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,
    pub environment: Option<String>,
}

impl StatsigOptions {
    pub fn new() -> Self {
        // simple_logger::init_with_level(log::Level::Debug).unwrap();

        Self::default()
    }

    // The builder method for more complex initialization
    pub fn builder() -> StatsigOptionsBuilder {
        StatsigOptionsBuilder::default()
    }
}

#[derive(Default)]
pub struct StatsigOptionsBuilder {
    specs_url: Option<String>,
    log_event_url: Option<String>,
    specs_adapter: Option<Arc<dyn SpecsAdapter>>,
    specs_sync_interval_ms: Option<u32>,
    event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
    event_logging_flush_interval_ms: Option<u32>,
    event_logging_max_queue_size: Option<u32>,
    environment: Option<String>,
}

impl StatsigOptionsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn specs_url(mut self, specs_url: Option<String>) -> Self {
        self.specs_url = specs_url;
        self
    }

    pub fn log_event_url(mut self, log_event_url: Option<String>) -> Self {
        self.log_event_url = log_event_url;
        self
    }

    pub fn specs_adapter(mut self, specs_adapter: Option<Arc<dyn SpecsAdapter>>) -> Self {
        self.specs_adapter = specs_adapter;
        self
    }

    pub fn specs_sync_interval_ms(mut self, specs_sync_interval_ms: Option<u32>) -> Self {
        self.specs_sync_interval_ms = specs_sync_interval_ms;
        self
    }

    pub fn event_logging_adapter(mut self, event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>) -> Self {
        self.event_logging_adapter = event_logging_adapter;
        self
    }

    pub fn event_logging_flush_interval_ms(mut self, event_logging_flush_interval_ms: Option<u32>) -> Self {
        self.event_logging_flush_interval_ms = event_logging_flush_interval_ms;
        self
    }

    pub fn event_logging_max_queue_size(mut self, event_logging_max_queue_size: Option<u32>) -> Self {
        self.event_logging_max_queue_size = event_logging_max_queue_size;
        self
    }

    pub fn environment(mut self, environment: Option<String>) -> Self {
        self.environment = environment;
        self
    }

    pub fn build(self) -> StatsigOptions {
        StatsigOptions {
            specs_url: self.specs_url,
            log_event_url: self.log_event_url,
            specs_adapter: self.specs_adapter,
            specs_sync_interval_ms: self.specs_sync_interval_ms,
            event_logging_adapter: self.event_logging_adapter,
            event_logging_flush_interval_ms: self.event_logging_flush_interval_ms,
            event_logging_max_queue_size: self.event_logging_max_queue_size,
            environment: self.environment,
        }
    }
}
