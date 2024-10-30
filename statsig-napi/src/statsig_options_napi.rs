use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use sigstat::{instance_store::INST_STORE, log_e, StatsigOptions};

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigOptionsRef {
  pub ref_id: String,
}

impl ObjectFinalize for AutoReleasingStatsigOptionsRef {
  fn finalize(self, _env: Env) -> napi::Result<()> {
    INST_STORE.remove(&self.ref_id);
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
  let ref_id = INST_STORE
    .add(StatsigOptions {
      environment,
      specs_url,
      specs_sync_interval_ms,
      log_event_url,
      event_logging_max_queue_size,
      event_logging_flush_interval_ms,
      ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
      log_e!("Failed to create StatsigOptions");
      "".to_string()
    });

  AutoReleasingStatsigOptionsRef { ref_id }
}
