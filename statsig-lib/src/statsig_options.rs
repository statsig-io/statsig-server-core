use crate::SpecsAdapter;
use std::sync::Arc;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::id_lists_adapter::IdListsAdapter;
use crate::output_logger::LogLevel;

#[derive(Clone, Default)]
pub struct StatsigOptions {
    pub specs_url: Option<String>,
    pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
    pub specs_sync_interval_ms: Option<u32>,

    pub log_event_url: Option<String>,
    pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub id_lists_url: Option<String>,
    pub id_lists_adapter: Option<Arc<dyn IdListsAdapter>>,
    pub id_lists_sync_interval_ms: Option<u32>,


    pub output_log_level: Option<LogLevel>,
    pub environment: Option<String>,
}

impl StatsigOptions {
    pub fn new() -> Self {
        Self::default()
    }

    // The builder method for more complex initialization
    pub fn builder() -> StatsigOptionsBuilder {
        StatsigOptionsBuilder::default()
    }
}

#[derive(Default)]
pub struct StatsigOptionsBuilder {
    inner: StatsigOptions,
}

impl StatsigOptionsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    // Specs

    pub fn specs_url(mut self, specs_url: Option<String>) -> Self {
        self.inner.specs_url = specs_url;
        self
    }

    pub fn specs_adapter(mut self, specs_adapter: Option<Arc<dyn SpecsAdapter>>) -> Self {
        self.inner.specs_adapter = specs_adapter;
        self
    }

    pub fn specs_sync_interval_ms(mut self, specs_sync_interval_ms: Option<u32>) -> Self {
        self.inner.specs_sync_interval_ms = specs_sync_interval_ms;
        self
    }

    // Event Logging

    pub fn log_event_url(mut self, log_event_url: Option<String>) -> Self {
        self.inner.log_event_url = log_event_url;
        self
    }

    pub fn event_logging_adapter(mut self, event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>) -> Self {
        self.inner.event_logging_adapter = event_logging_adapter;
        self
    }

    pub fn event_logging_flush_interval_ms(mut self, event_logging_flush_interval_ms: Option<u32>) -> Self {
        self.inner.event_logging_flush_interval_ms = event_logging_flush_interval_ms;
        self
    }

    pub fn event_logging_max_queue_size(mut self, event_logging_max_queue_size: Option<u32>) -> Self {
        self.inner.event_logging_max_queue_size = event_logging_max_queue_size;
        self
    }

    // ID Lists

    pub fn id_lists_url(mut self, id_lists_url: Option<String>) -> Self {
        self.inner.id_lists_url = id_lists_url;
        self
    }

    pub fn id_lists_adapter(mut self, id_lists_adapter: Option<Arc<dyn IdListsAdapter>>) -> Self {
        self.inner.id_lists_adapter = id_lists_adapter;
        self
    }

    pub fn id_lists_sync_interval_ms(mut self, id_lists_sync_interval_ms: Option<u32>) -> Self {
        self.inner.id_lists_sync_interval_ms = id_lists_sync_interval_ms;
        self
    }

    // Other

    pub fn environment(mut self, environment: Option<String>) -> Self {
        self.inner.environment = environment;
        self
    }

    pub fn output_log_level(mut self, output_log_level: Option<u32>) -> Self {
        if let Some(level) = output_log_level {
            self.inner.output_log_level = LogLevel::from_int(level);

        }
        self
    }

    pub fn build(self) -> StatsigOptions {
        self.inner
    }
}
