use napi_derive::napi;
use std::sync::{Arc, Weak};

use crate::{data_store_napi::DataStore, observability_client_napi::ObservabilityClient};
use statsig_rust::{
    data_store_interface::DataStoreTrait,
    statsig_types::OverrideAdapterType as OverrideAdapterTypeActual,
    ObservabilityClient as ObservabilityClientTrait, OverrideAdapter,
    SpecAdapterConfig as SpecAdapterConfigActual, StatsigLocalOverrideAdapter,
    StatsigOptions as StatsigOptionsActual, DEFAULT_INIT_TIMEOUT_MS,
};

#[napi(object)]
pub struct SpecAdapterConfig {
    #[napi(ts_type = "'data_store' | 'network_grpc_websocket' | 'network_http'")]
    pub adapter_type: String,
    pub specs_url: Option<String>,
    pub init_timeout_ms: i64,
}

#[napi]
pub enum OverrideAdapterType {
    LocalOverride,
}

impl From<OverrideAdapterType> for OverrideAdapterTypeActual {
    fn from(adapter_type: OverrideAdapterType) -> Self {
        match adapter_type {
            OverrideAdapterType::LocalOverride => OverrideAdapterTypeActual::LocalOverride,
        }
    }
}
#[napi(object)]
pub struct OverrideAdapterConfig {
    pub adapter_type: OverrideAdapterType,
    // TODO: add custom adapter interface support
}

#[napi(object, object_to_js = false)]
pub struct StatsigOptions {
    pub data_store: Option<DataStore>,
    pub disable_all_logging: Option<bool>,
    pub enable_id_lists: Option<bool>,
    pub enable_user_agent_parsing: Option<bool>,
    pub enable_country_lookup: Option<bool>,
    pub environment: Option<String>,

    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub fallback_to_statsig_api: Option<bool>,

    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,

    pub init_timeout_ms: Option<i64>,
    pub log_event_url: Option<String>,
    pub observability_client: Option<ObservabilityClient>,
    #[napi(ts_type = "'none' | 'debug' | 'info' | 'warn' | 'error'")]
    pub output_log_level: Option<String>,

    pub spec_adapters_config: Option<Vec<SpecAdapterConfig>>,
    pub specs_url: Option<String>,
    pub specs_sync_interval_ms: Option<u32>,

    pub service_name: Option<String>,
    pub override_adapter_config: Option<Vec<OverrideAdapterConfig>>,
}

impl StatsigOptions {
    /**
     * There is some reference capture issue around ObservabilityClient and StatsigOptions.
     * By storing the ObservabilityClient in a Weak reference and having the strong
     * Arc reference be owned by StatsigNapi, we can avoid the issue.
     */
    pub fn safe_convert_to_inner(
        mut self,
    ) -> (
        Option<Arc<StatsigOptionsActual>>,
        Option<Arc<ObservabilityClient>>,
    ) {
        let obs_client = self.observability_client.take().map(Arc::new);

        let mut weak_obs_client: Option<Weak<dyn ObservabilityClientTrait>> = None;
        if let Some(obs_client) = &obs_client {
            weak_obs_client =
                Some(Arc::downgrade(obs_client) as Weak<dyn ObservabilityClientTrait>);
        }

        self.observability_client = None;

        let inner = StatsigOptionsActual {
            data_store: self
                .data_store
                .map(|store| Arc::new(store) as Arc<dyn DataStoreTrait>),
            enable_id_lists: self.enable_id_lists,
            enable_user_agent_parsing: self.enable_user_agent_parsing,
            enable_country_lookup: self.enable_country_lookup,
            environment: self.environment,
            fallback_to_statsig_api: self.fallback_to_statsig_api,
            id_lists_sync_interval_ms: self.id_lists_sync_interval_ms,
            id_lists_url: self.id_lists_url,
            init_timeout_ms: self.init_timeout_ms.map(|ms| ms as u64),
            log_event_url: self.log_event_url,
            observability_client: weak_obs_client,
            output_log_level: self.output_log_level.map(|s| s.as_str().into()),
            specs_sync_interval_ms: self.specs_sync_interval_ms,
            specs_url: self.specs_url,
            spec_adapters_config: self
                .spec_adapters_config
                .map(|configs| configs.into_iter().map(|c| c.into()).collect()),
            service_name: self.service_name,
            disable_all_logging: self.disable_all_logging,
            event_logging_flush_interval_ms: self.event_logging_flush_interval_ms,
            event_logging_max_queue_size: self.event_logging_max_queue_size,
            override_adapter: Some(create_local_overrides(self.override_adapter_config)),
            ..Default::default()
        };

        (Some(Arc::new(inner)), obs_client)
    }
}

impl From<SpecAdapterConfig> for SpecAdapterConfigActual {
    fn from(val: SpecAdapterConfig) -> Self {
        SpecAdapterConfigActual {
            adapter_type: val.adapter_type.into(),
            specs_url: val.specs_url,
            init_timeout_ms: u64::try_from(val.init_timeout_ms).unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
        }
    }
}

fn create_local_overrides(
    _override_adapter_config: Option<Vec<OverrideAdapterConfig>>,
) -> Arc<dyn OverrideAdapter> {
    // Default to the local override adapter for now.
    Arc::new(StatsigLocalOverrideAdapter::new())
}
