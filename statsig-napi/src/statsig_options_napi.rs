use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use sigstat::{instance_store::OPTIONS_INSTANCES, StatsigOptions};

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigOptionsRef {
    pub value: i32,
}

impl ObjectFinalize for AutoReleasingStatsigOptionsRef {
    fn finalize(self, _env: Env) -> napi::Result<()> {
        OPTIONS_INSTANCES.release(self.value);
        Ok(())
    }
}

#[napi]
pub fn statsig_options_create(
    environment: Option<String>,
    specs_url: Option<String>,
    specs_sync_interval_ms: Option<u32>,
    log_event_url: Option<String>,
    event_logging_max_queue_size: Option<u32>,
    event_logging_flush_interval_ms: Option<u32>,
) -> AutoReleasingStatsigOptionsRef {
    // pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
    // pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,

    AutoReleasingStatsigOptionsRef {
        value: OPTIONS_INSTANCES.add(StatsigOptions {
            environment,
            specs_url,
            specs_sync_interval_ms,
            log_event_url,
            event_logging_max_queue_size,
            event_logging_flush_interval_ms,
            ..StatsigOptions::new()
        })
    }
}