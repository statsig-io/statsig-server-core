use rustler::NifStruct;
use statsig_rust::{
    data_store_interface::DataStoreTrait, statsig_options::StatsigOptions as StatsigOptionsActual,
    SpecAdapterConfig as SpecAdapterConfigActual, DEFAULT_INIT_TIMEOUT_MS,
};
use std::sync::Arc;

use crate::data_store_nfi::{ElixirDataStore, StatsigDataStoreReference};

#[derive(NifStruct)]
#[module = "Statsig.SpecAdapterConfig"]
pub struct SpecAdapterConfig {
    pub adapter_type: String,
    pub specs_url: Option<String>,
    pub init_timeout_ms: Option<u64>,
    pub authentication_mode: Option<String>,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub domain_name: Option<String>,
}

impl From<SpecAdapterConfig> for SpecAdapterConfigActual {
    fn from(config: SpecAdapterConfig) -> Self {
        SpecAdapterConfigActual {
            adapter_type: config.adapter_type.into(),
            specs_url: config.specs_url,
            init_timeout_ms: config.init_timeout_ms.unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
            authentication_mode: config.authentication_mode,
            ca_cert_path: config.ca_cert_path,
            client_cert_path: config.client_cert_path,
            client_key_path: config.client_key_path,
            domain_name: config.domain_name,
        }
    }
}

#[derive(NifStruct)]
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
    pub spec_adapter_configs: Option<Vec<SpecAdapterConfig>>,

    pub enable_id_lists: Option<bool>,
    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,

    pub disable_all_logging: Option<bool>,
    pub disable_country_lookup: Option<bool>,
    pub disable_network: Option<bool>, // Disable all out-going network including get configs, log_events...

    pub wait_for_country_lookup_init: Option<bool>,
    pub wait_for_user_agent_init: Option<bool>,

    pub use_third_party_ua_parser: Option<bool>,
    pub disable_disk_access: Option<bool>,
    pub data_store: Option<StatsigDataStoreReference>,
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
            spec_adapters_config: config
                .spec_adapter_configs
                .map(|configs| configs.into_iter().map(|c| c.into()).collect()),
            log_event_url: config.log_event_url,
            output_log_level: config.output_log_level.map(|o| o.as_str().into()),
            disable_all_logging: config.disable_all_logging,
            disable_country_lookup: config.disable_country_lookup,
            disable_network: config.disable_network,
            wait_for_country_lookup_init: config.wait_for_country_lookup_init,
            wait_for_user_agent_init: config.wait_for_user_agent_init,
            use_third_party_ua_parser: config.use_third_party_ua_parser,
            disable_disk_access: config.disable_disk_access,
            id_lists_sync_interval_ms: config.id_lists_sync_interval_ms,
            #[allow(deprecated)]
            event_logging_flush_interval_ms: config.event_logging_flush_interval_ms,
            init_timeout_ms: config.init_timeout_ms,
            data_store: config.data_store.map(|reference| {
                Arc::new(ElixirDataStore::new(reference.pid)) as Arc<dyn DataStoreTrait>
            }),
            ..StatsigOptionsActual::default()
        }
    }
}
