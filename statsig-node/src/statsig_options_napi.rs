use napi::bindgen_prelude::{ClassInstance, Either4};
use napi_derive::napi;
use serde_json::Value;
use statsig_rust::{log_e, PersistentStorage};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use crate::persistent_storage_napi::PersistentStorageNapi;
use crate::statsig_user_napi::StatsigUser;
use crate::{
    data_store_napi::DataStore, observability_client_napi::ObservabilityClient,
    output_logger_napi::OutputLoggerProvider,
};
use statsig_rust::{
    console_capture::console_capture_options::ConsoleCaptureOptions as ConsoleCaptureOptionsActual,
    console_capture::console_log_line_levels::StatsigLogLineLevel,
    data_store_interface::DataStoreTrait,
    networking::proxy_config::ProxyConfig as ProxyConfigActual, output_logger::OutputLogProvider,
    statsig_types::OverrideAdapterType as OverrideAdapterTypeActual, DynamicValue,
    ObservabilityClient as ObservabilityClientTrait, OverrideAdapter,
    SpecAdapterConfig as SpecAdapterConfigActual, StatsigLocalOverrideAdapter,
    StatsigOptions as StatsigOptionsActual, DEFAULT_INIT_TIMEOUT_MS,
};

type ValidPrimitives = Either4<String, f64, bool, Vec<Value>>;

const TAG: &str = "StatsigOptionsNapi";
#[napi(object)]
pub struct ProxyConfig {
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    pub proxy_auth: Option<String>,
    pub proxy_protocol: Option<String>,
}

impl From<ProxyConfig> for ProxyConfigActual {
    fn from(napi: ProxyConfig) -> Self {
        Self {
            proxy_host: napi.proxy_host,
            proxy_port: napi.proxy_port,
            proxy_auth: napi.proxy_auth,
            proxy_protocol: napi.proxy_protocol,
        }
    }
}

#[napi(object, object_to_js = false)]
pub struct ConsoleCaptureOptions<'env> {
    pub enabled: bool,
    #[napi(ts_type = "Array<'trace' | 'debug' | 'log' | 'info' | 'warn' | 'error'>")]
    pub log_levels: Option<Vec<String>>,
    pub user: Option<ClassInstance<'env, StatsigUser>>,
    pub max_keys: Option<i64>,
    pub max_depth: Option<i64>,
    pub max_length: Option<i64>,
}

impl<'env> From<ConsoleCaptureOptions<'env>> for ConsoleCaptureOptionsActual {
    fn from(napi: ConsoleCaptureOptions<'env>) -> Self {
        let mut parsed_levels = Vec::new();
        if let Some(levels) = &napi.log_levels {
            for level in levels {
                match StatsigLogLineLevel::from_string(level) {
                    Some(parsed) => parsed_levels.push(parsed),
                    None => log_e!(TAG, "Could not parse log level: {}", level),
                }
            }
        }

        Self {
            enabled: napi.enabled,
            log_levels: Some(parsed_levels),
            user: napi.user.map(|user| user.as_inner().clone()),
            max_keys: napi.max_keys.map(|max_keys| max_keys as u64),
            max_depth: napi.max_depth.map(|max_depth| max_depth as u64),
            max_length: napi.max_length.map(|max_length| max_length as u64),
        }
    }
}

#[napi(object)]
pub struct SpecAdapterConfig {
    #[napi(ts_type = "'data_store' | 'network_grpc_websocket' | 'network_http'")]
    pub adapter_type: String,
    pub specs_url: Option<String>,
    pub init_timeout_ms: i64,

    #[napi(ts_type = "'none' | 'tls' | 'mtls' | undefined | null")]
    pub authentication_mode: Option<String>,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub domain_name: Option<String>,
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
pub struct StatsigOptions<'a> {
    pub data_store: Option<DataStore>,
    #[napi(ts_type = "'v2' | 'v3'")]
    pub data_store_key_schema_version: Option<String>,

    pub disable_all_logging: Option<bool>,
    pub disable_country_lookup: Option<bool>,
    pub disable_network: Option<bool>,

    pub enable_id_lists: Option<bool>,
    pub enable_dcs_deltas: Option<bool>,
    pub environment: Option<String>,

    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub fallback_to_statsig_api: Option<bool>,

    #[napi(
        ts_type = "Record<string, string | number | boolean | Array<string | number | boolean>>"
    )]
    pub global_custom_fields: Option<HashMap<String, ValidPrimitives>>,

    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,

    pub init_timeout_ms: Option<i64>,
    pub log_event_url: Option<String>,
    pub observability_client: Option<ObservabilityClient>,

    #[napi(ts_type = "'none' | 'debug' | 'info' | 'warn' | 'error'")]
    pub output_log_level: Option<String>,
    #[napi(ts_type = "OutputLoggerProvider")]
    pub output_logger_provider: Option<OutputLoggerProvider>,

    #[napi(ts_type = "'gzip' | 'dictionary'")]
    pub config_compression_mode: Option<String>,
    pub override_adapter_config: Option<Vec<OverrideAdapterConfig>>,
    pub service_name: Option<String>,
    #[napi(ts_type = "PersistentStorage")]
    pub persistent_storage: Option<PersistentStorageNapi>,

    pub spec_adapters_config: Option<Vec<SpecAdapterConfig>>,
    pub specs_sync_interval_ms: Option<u32>,
    pub specs_url: Option<String>,

    pub wait_for_country_lookup_init: Option<bool>,
    pub wait_for_user_agent_init: Option<bool>,

    pub proxy_config: Option<ProxyConfig>,

    pub console_capture_options: Option<ConsoleCaptureOptions<'a>>,

    pub use_third_party_ua_parser: Option<bool>,

    pub experimental_flags: Option<HashSet<String>>,
}

impl StatsigOptions<'_> {
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
            data_store_key_schema_version: self
                .data_store_key_schema_version
                .map(|v| v.as_str().into()),
            enable_id_lists: self.enable_id_lists,
            enable_dcs_deltas: self.enable_dcs_deltas,
            wait_for_country_lookup_init: self.wait_for_country_lookup_init,
            wait_for_user_agent_init: self.wait_for_country_lookup_init,
            environment: self.environment,
            fallback_to_statsig_api: self.fallback_to_statsig_api,
            id_lists_sync_interval_ms: self.id_lists_sync_interval_ms,
            id_lists_url: self.id_lists_url,
            init_timeout_ms: self.init_timeout_ms.map(|ms| ms as u64),
            log_event_url: self.log_event_url,
            observability_client: weak_obs_client,
            output_log_level: self.output_log_level.map(|s| s.as_str().into()),
            output_logger_provider: self
                .output_logger_provider
                .map(|ol| Arc::new(ol) as Arc<dyn OutputLogProvider>),
            config_compression_mode: self.config_compression_mode.map(|s| s.as_str().into()),
            specs_sync_interval_ms: self.specs_sync_interval_ms,
            specs_url: self.specs_url,
            spec_adapters_config: self
                .spec_adapters_config
                .map(|configs| configs.into_iter().map(|c| c.into()).collect()),
            service_name: self.service_name,
            disable_all_logging: self.disable_all_logging,
            disable_network: self.disable_network,
            event_logging_max_queue_size: self.event_logging_max_queue_size,
            override_adapter: Some(create_local_overrides(self.override_adapter_config)),
            global_custom_fields: Self::convert_to_dynamic_value_map(self.global_custom_fields),
            disable_country_lookup: self.disable_country_lookup,
            proxy_config: self.proxy_config.map(|p| p.into()),
            console_capture_options: self.console_capture_options.map(|c| c.into()),
            use_third_party_ua_parser: self.use_third_party_ua_parser,
            persistent_storage: self
                .persistent_storage
                .map(|s| Arc::new(s) as Arc<dyn PersistentStorage>),
            experimental_flags: self.experimental_flags,

            // empty fields
            event_logging_adapter: None,
            #[allow(deprecated)]
            event_logging_flush_interval_ms: None,
            event_logging_max_pending_batch_queue_size: None,
            id_lists_adapter: None,
            specs_adapter: None,
            disable_disk_access: None,
        };

        (Some(Arc::new(inner)), obs_client)
    }

    fn convert_to_dynamic_value_map(
        map: Option<HashMap<String, ValidPrimitives>>,
    ) -> Option<HashMap<String, DynamicValue>> {
        let map = match map {
            Some(map) => map,
            _ => return None,
        };

        let mut converted: HashMap<String, DynamicValue> = HashMap::new();

        for (key, value) in map {
            match value {
                Either4::A(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::B(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::C(value) => converted.insert(key, DynamicValue::from(value)),
                Either4::D(value) => converted.insert(key, DynamicValue::from(value)),
            };
        }

        Some(converted)
    }
}

impl From<SpecAdapterConfig> for SpecAdapterConfigActual {
    fn from(val: SpecAdapterConfig) -> Self {
        SpecAdapterConfigActual {
            adapter_type: val.adapter_type.into(),
            specs_url: val.specs_url,
            init_timeout_ms: u64::try_from(val.init_timeout_ms).unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
            authentication_mode: val.authentication_mode,
            ca_cert_path: val.ca_cert_path,
            client_cert_path: val.client_cert_path,
            client_key_path: val.client_key_path,
            domain_name: val.domain_name,
        }
    }
}

fn create_local_overrides(
    _override_adapter_config: Option<Vec<OverrideAdapterConfig>>,
) -> Arc<dyn OverrideAdapter> {
    // Default to the local override adapter for now.
    Arc::new(StatsigLocalOverrideAdapter::new())
}
