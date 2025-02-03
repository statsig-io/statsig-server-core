use pyo3::prelude::*;
use sigstat::{output_logger::LogLevel, StatsigOptions};

#[pyclass(name = "StatsigOptions")]
pub struct StatsigOptionsPy {
    #[pyo3(get, set)]
    pub specs_url: Option<String>,
    #[pyo3(get, set)]
    pub specs_sync_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub init_timeout_ms: Option<u64>,
    #[pyo3(get, set)]
    pub log_event_url: Option<String>,
    #[pyo3(get, set)]
    pub disable_all_logging: Option<bool>,
    #[pyo3(get, set)]
    pub event_logging_flush_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub event_logging_max_queue_size: Option<u32>,
    #[pyo3(get, set)]
    pub enable_id_lists: Option<bool>,
    #[pyo3(get, set)]
    pub id_lists_url: Option<String>,
    #[pyo3(get, set)]
    pub id_lists_sync_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub fallback_to_statsig_api: Option<bool>,
    #[pyo3(get, set)]
    pub environment: Option<String>,
    #[pyo3(get, set)]
    pub output_log_level: Option<String>,
}

#[pymethods]
impl StatsigOptionsPy {
    #[new]
    pub fn new() -> Self {
        Self {
            specs_url: None,
            specs_sync_interval_ms: None,
            init_timeout_ms: None,
            log_event_url: None,
            disable_all_logging: None,
            event_logging_flush_interval_ms: None,
            event_logging_max_queue_size: None,
            enable_id_lists: None,
            id_lists_url: None,
            id_lists_sync_interval_ms: None,
            fallback_to_statsig_api: None,
            environment: None,
            output_log_level: None,
        }
    }
}

impl Default for StatsigOptionsPy {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&StatsigOptionsPy> for StatsigOptions {
    fn from(val: &StatsigOptionsPy) -> StatsigOptions {
        StatsigOptions {
            specs_url: val.specs_url.clone(),
            specs_adapter: None,
            specs_sync_interval_ms: val.specs_sync_interval_ms,
            init_timeout_ms: val.init_timeout_ms,
            data_store: None,
            spec_adapters_config: None,
            log_event_url: val.log_event_url.clone(),
            disable_all_logging: val.disable_all_logging,
            event_logging_adapter: None,
            event_logging_flush_interval_ms: val.event_logging_flush_interval_ms,
            event_logging_max_queue_size: val.event_logging_max_queue_size,
            enable_id_lists: val.enable_id_lists,
            id_lists_url: val.id_lists_url.clone(),
            id_lists_sync_interval_ms: val.id_lists_sync_interval_ms,
            fallback_to_statsig_api: val.fallback_to_statsig_api,
            environment: val.environment.clone(),
            id_lists_adapter: None,
            override_adapter: None,
            output_log_level: val
                .output_log_level
                .as_ref()
                .map(|level| LogLevel::from(level.as_str())),
            observability_client: None,
            service_name: None,
        }
    }
}
