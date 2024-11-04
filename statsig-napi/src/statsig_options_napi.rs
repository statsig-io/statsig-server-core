use napi::bindgen_prelude::ObjectFinalize;
use napi::Env;
use napi_derive::napi;
use sigstat::{
  instance_store::INST_STORE, log_e, SpecAdapterConfig, AdapterType,
  StatsigOptions, DEFAULT_INIT_TIMEOUT_MS,
};

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
  spec_adapters_config: Option<Vec<SpecAdapterConfigNapi>>,
) -> AutoReleasingStatsigOptionsRef {
  // pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
  // pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
  let spec_adapters_config: Option<Vec<SpecAdapterConfig>> = spec_adapters_config.map(|unwrapped| {
    unwrapped
      .into_iter()
      .map(|config| config.into())
      .collect()
  });
  let ref_id = INST_STORE
    .add(StatsigOptions {
      environment,
      specs_url,
      specs_sync_interval_ms,
      log_event_url,
      event_logging_max_queue_size,
      event_logging_flush_interval_ms,
      spec_adapters_config: spec_adapters_config,
      ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
      log_e!("Failed to create StatsigOptions");
      "".to_string()
    });

  AutoReleasingStatsigOptionsRef { ref_id }
}
#[napi(object)]
pub struct SpecAdapterConfigNapi {
  pub adapter_type: AdapterTypeNapi,
  pub specs_url: String,
  pub init_timeout_ms: i64,
}

impl Into<SpecAdapterConfig> for SpecAdapterConfigNapi {
  fn into(self) -> SpecAdapterConfig {
    SpecAdapterConfig {
      adapter_type: self.adapter_type.into(),
      specs_url: self.specs_url,
      init_timeout_ms: u64::try_from(self.init_timeout_ms).unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
    }
  }
}

#[napi]
pub enum AdapterTypeNapi {
  NetworkHttp,
  NetworkGrpcWebsocket,
}

impl Into<AdapterType> for AdapterTypeNapi {
  fn into(self) -> AdapterType {
    match self {
      AdapterTypeNapi::NetworkGrpcWebsocket => AdapterType::NetworkGrpcWebsocket,
      AdapterTypeNapi::NetworkHttp => AdapterType::NetworkHttp,
    }
  }
}
