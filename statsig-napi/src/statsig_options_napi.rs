use std::sync::Arc;

use napi::{bindgen_prelude::ObjectFinalize, JsObject};
use napi::Env;
use napi_derive::napi;
use sigstat::data_store_interface::DataStoreTrait;
use sigstat::IObservabilityClient;

use sigstat::{
  instance_store::INST_STORE, log_e, SpecAdapterConfig, SpecsAdapterType, StatsigOptions,
  DEFAULT_INIT_TIMEOUT_MS,
};

use crate::data_store::DataStore;


const TAG: &str = "StatsigOptionsNapi";
use crate::observability_client::{self, ObservabilityClient};

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
  data_store: Option<JsObject>,
  specs_url: Option<String>,
  specs_sync_interval_ms: Option<u32>,
  log_event_url: Option<String>,
  event_logging_max_queue_size: Option<u32>,
  event_logging_flush_interval_ms: Option<u32>,
  spec_adapters_config: Option<Vec<SpecAdapterConfigNapi>>,
  observability_client: Option<JsObject>,
) -> AutoReleasingStatsigOptionsRef {
  let data_store: Option<Arc<dyn DataStoreTrait + 'static>> = if let Some(store_unwrapped) = data_store {
      Some(Arc::new(DataStore::new(store_unwrapped)))
  } else {
    None
  };
  let observability_client: Option<Arc<dyn IObservabilityClient>> = if let Some(ob) = observability_client {
    Some(Arc::new(ObservabilityClient::new(ob)))
  } else {
    None
  };
  // pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
  // pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
  let spec_adapters_config: Option<Vec<SpecAdapterConfig>> = spec_adapters_config
    .map(|unwrapped| unwrapped.into_iter().map(|config| config.into()).collect());

    let ref_id = INST_STORE
    .add(StatsigOptions {
      environment,
      specs_url,
      specs_sync_interval_ms,
      log_event_url,
      event_logging_max_queue_size,
      event_logging_flush_interval_ms,
      spec_adapters_config: spec_adapters_config,
      data_store,
      observability_client,
      ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
      log_e!(TAG, "Failed to create StatsigOptions");
      "".to_string()
    });

  AutoReleasingStatsigOptionsRef { ref_id }
}
#[napi(object)]
pub struct SpecAdapterConfigNapi {
  pub adapter_type: SpecsAdapterTypeNapi,
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
pub enum SpecsAdapterTypeNapi {
  NetworkHttp,
  NetworkGrpcWebsocket,
}

impl Into<SpecsAdapterType> for SpecsAdapterTypeNapi {
  fn into(self) -> SpecsAdapterType {
    match self {
      SpecsAdapterTypeNapi::NetworkGrpcWebsocket => SpecsAdapterType::NetworkGrpcWebsocket,
      SpecsAdapterTypeNapi::NetworkHttp => SpecsAdapterType::NetworkHttp,
    }
  }
}
