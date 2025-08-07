use rustler::NifStruct;
use serde::Serialize;
use statsig_rust::statsig_options::StatsigOptions as StatsigOptionsActual;

#[derive(NifStruct, Serialize)]
#[module = "Statsig.Options"]
pub struct StatsigOptions {
    pub environment: Option<String>,
    pub output_log_level: Option<String>,
    pub init_timeout_ms: Option<u64>,
    pub fallback_to_statsig_api: Option<bool>,

    pub log_event_url: Option<String>,
    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub specs_sync_interval_ms: Option<u32>,
    pub specs_url: Option<String>,

    pub enable_id_lists: Option<bool>,
    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,
}

impl From<StatsigOptions> for StatsigOptionsActual {
    fn from(config: StatsigOptions) -> Self {
        StatsigOptionsActual {
            environment: config.environment,
            enable_id_lists: config.enable_id_lists,
            id_lists_url: config.id_lists_url,
            event_logging_max_queue_size: config.event_logging_max_queue_size,
            fallback_to_statsig_api: config.fallback_to_statsig_api,
            specs_sync_interval_ms: config.specs_sync_interval_ms,
            specs_url: config.specs_url,
            log_event_url: config.log_event_url,
            ..StatsigOptionsActual::default()
        }
    }
}
