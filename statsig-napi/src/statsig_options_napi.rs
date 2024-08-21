use napi_derive::napi;
use statsig::{instance_store::OPTIONS_INSTANCES, StatsigOptions};

#[napi]
pub fn statsig_options_create(
  environment: Option<String>,
  specs_url: Option<String>,
  log_event_url: Option<String>,
) -> i32 {
  OPTIONS_INSTANCES.add(StatsigOptions {
    environment,
    specs_url,
    log_event_url,
    ..StatsigOptions::new()
  })
}

#[napi]
pub fn statsig_options_release(user_ref: i32) {
  OPTIONS_INSTANCES.release(user_ref)
}
