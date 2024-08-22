use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use statsig::{instance_store::OPTIONS_INSTANCES, StatsigOptions};

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
    log_event_url: Option<String>,
) -> AutoReleasingStatsigOptionsRef {
    AutoReleasingStatsigOptionsRef {
        value: OPTIONS_INSTANCES.add(StatsigOptions {
            environment,
            specs_url,
            log_event_url,
            ..StatsigOptions::new()
        })
    }
}