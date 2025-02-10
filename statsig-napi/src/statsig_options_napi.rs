use std::sync::{Arc, Mutex, Weak};

use napi::Env;
use napi::{bindgen_prelude::ObjectFinalize, JsObject};
use napi_derive::napi;
use sigstat::data_store_interface::DataStoreTrait;
use sigstat::ObservabilityClient;

use sigstat::{
  instance_store::INST_STORE, log_e, SpecAdapterConfig, SpecsAdapterType, StatsigOptions,
  DEFAULT_INIT_TIMEOUT_MS,
};

use crate::data_store_napi::DataStoreNapi;
use crate::observability_client_napi::ObservabilityClientNapi;

const TAG: &str = "StatsigOptionsNapi";

#[napi(custom_finalize)]
pub struct AutoReleasingStatsigOptionsRef {
  pub ref_id: String,

  observability_client: Mutex<Option<Arc<dyn ObservabilityClient>>>,
}

impl ObjectFinalize for AutoReleasingStatsigOptionsRef {
  fn finalize(self, _env: Env) -> napi::Result<()> {
    INST_STORE.remove(&self.ref_id);
    drop(self.observability_client);
    Ok(())
  }
}

#[napi]
#[allow(clippy::too_many_arguments)] // todo: refactor this to use a builder pattern or config object
pub fn statsig_options_create(
  environment: Option<String>,
  data_store: Option<JsObject>,
  specs_url: Option<String>,
  specs_sync_interval_ms: Option<u32>,
  log_event_url: Option<String>,
  event_logging_max_queue_size: Option<u32>,
  event_logging_flush_interval_ms: Option<u32>,
  spec_adapters_config: Option<Vec<SpecAdapterConfigNapi>>,
  output_log_level: Option<u32>,
  observability_client: Option<JsObject>,
  service_name: Option<String>,
) -> AutoReleasingStatsigOptionsRef {
  let data_store: Option<Arc<dyn DataStoreTrait + 'static>> =
    if let Some(store_unwrapped) = data_store {
      Some(Arc::new(DataStoreNapi::new(store_unwrapped)))
    } else {
      None
    };

  let (observability_client, weak_observability_client) = match observability_client {
    Some(ob) => {
      let obs_client_arc: Arc<dyn ObservabilityClient> = Arc::new(ObservabilityClientNapi::new(ob));
      let weak_obs_client: Weak<dyn ObservabilityClient> = Arc::downgrade(&obs_client_arc);
      (Some(obs_client_arc), Some(weak_obs_client))
    }
    None => (None, None),
  };

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
      spec_adapters_config,
      data_store,
      observability_client: weak_observability_client,
      output_log_level: output_log_level.map(|l| l.into()),
      service_name,
      ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
      log_e!(TAG, "Failed to create StatsigOptions");
      "".to_string()
    });

  AutoReleasingStatsigOptionsRef {
    ref_id,
    observability_client: Mutex::new(observability_client),
  }
}
#[napi(object)]
pub struct SpecAdapterConfigNapi {
  pub adapter_type: SpecsAdapterTypeNapi,
  pub specs_url: Option<String>,
  pub init_timeout_ms: i64,
}

impl From<SpecAdapterConfigNapi> for SpecAdapterConfig {
  fn from(val: SpecAdapterConfigNapi) -> Self {
    SpecAdapterConfig {
      adapter_type: val.adapter_type.into(),
      specs_url: val.specs_url,
      init_timeout_ms: u64::try_from(val.init_timeout_ms).unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
    }
  }
}

#[napi]
pub enum SpecsAdapterTypeNapi {
  NetworkHttp,
  NetworkGrpcWebsocket,
  DataStore,
}

impl From<SpecsAdapterTypeNapi> for SpecsAdapterType {
  fn from(val: SpecsAdapterTypeNapi) -> SpecsAdapterType {
    match val {
      SpecsAdapterTypeNapi::NetworkGrpcWebsocket => SpecsAdapterType::NetworkGrpcWebsocket,
      SpecsAdapterTypeNapi::NetworkHttp => SpecsAdapterType::NetworkHttp,
      SpecsAdapterTypeNapi::DataStore => SpecsAdapterType::DataStore,
    }
  }
}
