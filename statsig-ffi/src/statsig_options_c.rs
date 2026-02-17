use std::{
    collections::HashSet,
    os::raw::{c_char, c_int},
    sync::Arc,
};

use crate::{
    data_store_c::DataStoreC,
    ffi_utils::{c_char_to_string, c_int_to_u32, c_int_to_u64, extract_opt_bool, SafeOptBool},
    function_based_event_logging_adapter_c::FunctionBasedEventLoggingAdapterC,
    function_based_specs_adapter_c::FunctionBasedSpecsAdapterC,
    observability_client_c::ObservabilityClientC,
    persistent_storage_c::PersistentStorageC,
};
use statsig_rust::{
    data_store_interface::DataStoreTrait, log_e, networking::proxy_config::ProxyConfig,
    output_logger::LogLevel, DynamicValue, EventLoggingAdapter, InstanceRegistry,
    ObservabilityClient, PersistentStorage, SpecAdapterConfig, SpecsAdapter,
    StatsigLocalFileEventLoggingAdapter, StatsigLocalFileSpecsAdapter, StatsigOptions,
};
use std::collections::HashMap;
use std::sync::Weak;

const TAG: &str = "StatsigOptionsC";

#[derive(Default, serde::Deserialize, Debug)]
pub struct StatsigOptionsData {
    config_compression_mode: Option<String>,
    data_store_ref: Option<u64>,
    data_store_key_schema_version: Option<String>,
    disable_all_logging: Option<bool>,
    disable_country_lookup: Option<bool>,
    disable_network: Option<bool>,
    disable_disk_access: Option<bool>,
    disable_user_agent_parsing: Option<bool>,
    enable_id_lists: Option<bool>,
    enable_dcs_deltas: Option<bool>,
    environment: Option<String>,
    event_logging_adapter_ref: Option<u64>,
    event_logging_max_pending_batch_queue_size: Option<u32>,
    event_logging_max_queue_size: Option<u32>,
    fallback_to_statsig_api: Option<bool>,
    global_custom_fields: Option<HashMap<String, DynamicValue>>,
    id_lists_sync_interval_ms: Option<u32>,
    id_lists_url: Option<String>,
    init_timeout_ms: Option<u64>,
    log_event_url: Option<String>,
    observability_client_ref: Option<u64>,
    output_log_level: Option<String>,
    persistent_storage_ref: Option<u64>,
    proxy_host: Option<String>,
    proxy_port: Option<u16>,
    proxy_auth: Option<String>,
    proxy_protocol: Option<String>,
    service_name: Option<String>,
    specs_adapter_ref: Option<u64>,
    // -- START STATSIG FORWARD PROXY CONFIG --
    spec_adapter_type: Option<String>,
    spec_adapter_url: Option<String>,
    spec_adapter_init_timeout_ms: Option<u64>,
    spec_adapter_authentication_mode: Option<String>,
    spec_adapter_ca_cert_path: Option<String>,
    spec_adapter_client_cert_path: Option<String>,
    spec_adapter_client_key_path: Option<String>,
    spec_adapter_domain_name: Option<String>,
    // -- END STATSIG FORWARD PROXY CONFIG --
    specs_sync_interval_ms: Option<u32>,
    specs_url: Option<String>,
    use_third_party_ua_parser: Option<bool>,
    wait_for_country_lookup_init: Option<bool>,
    wait_for_user_agent_init: Option<bool>,
    experimental_flags: Option<HashSet<String>>,
}

impl From<StatsigOptionsData> for StatsigOptions {
    fn from(data: StatsigOptionsData) -> Self {
        let data_store = match data.data_store_ref {
            Some(ds_ref) => try_get_data_store(ds_ref),
            None => None,
        };

        let event_logging_adapter = match data.event_logging_adapter_ref {
            Some(ela_ref) => try_get_event_logging_adapter(ela_ref),
            None => None,
        };

        let observability_client = match data.observability_client_ref {
            Some(oc_ref) => try_get_observability_client(oc_ref),
            None => None,
        };

        let persistent_storage = match data.persistent_storage_ref {
            Some(ps_ref) => try_get_persistent_storage(ps_ref),
            None => None,
        };

        let specs_adapter = match data.specs_adapter_ref {
            Some(sa_ref) => try_get_specs_adapter(sa_ref),
            None => None,
        };

        let data_store_key_schema_version = data
            .data_store_key_schema_version
            .map(|v| v.as_str().into());

        let output_log_level = data
            .output_log_level
            .map(|level| LogLevel::from(level.as_str()));

        let config_compression_mode = data
            .config_compression_mode
            .map(|mode| mode.as_str().into());

        let event_logging_max_pending_batch_queue_size =
            data.event_logging_max_pending_batch_queue_size;

        let proxy_config = create_proxy_config(
            data.proxy_host,
            data.proxy_port,
            data.proxy_auth,
            data.proxy_protocol,
        );

        let spec_adapters_config = create_spec_adapters_config(
            data.spec_adapter_type,
            data.spec_adapter_url,
            data.spec_adapter_init_timeout_ms,
            data.spec_adapter_authentication_mode,
            data.spec_adapter_ca_cert_path,
            data.spec_adapter_client_cert_path,
            data.spec_adapter_client_key_path,
            data.spec_adapter_domain_name,
        );

        // please keep sorted alphabetically
        Self {
            console_capture_options: None,
            config_compression_mode,
            data_store,
            data_store_key_schema_version,
            disable_all_logging: data.disable_all_logging,
            disable_disk_access: data.disable_disk_access,
            disable_country_lookup: data.disable_country_lookup,
            disable_network: data.disable_network,
            enable_dcs_deltas: data.enable_dcs_deltas,
            enable_id_lists: data.enable_id_lists,
            environment: data.environment,
            event_logging_adapter,
            #[allow(deprecated)]
            event_logging_flush_interval_ms: None, // Deprecated
            event_logging_max_pending_batch_queue_size,
            event_logging_max_queue_size: data.event_logging_max_queue_size,
            fallback_to_statsig_api: data.fallback_to_statsig_api,
            global_custom_fields: data.global_custom_fields,
            id_lists_adapter: None, // todo: add support for id lists adapter
            id_lists_sync_interval_ms: data.id_lists_sync_interval_ms,
            id_lists_url: data.id_lists_url,
            init_timeout_ms: data.init_timeout_ms,
            log_event_url: data.log_event_url,
            observability_client,
            output_log_level,
            output_logger_provider: None, // todo: add support for output logger provider
            override_adapter: None,       // todo: add support for override adapter
            persistent_storage,
            proxy_config,
            service_name: data.service_name,
            spec_adapters_config,
            specs_adapter,
            specs_sync_interval_ms: data.specs_sync_interval_ms,
            specs_url: data.specs_url,
            use_third_party_ua_parser: data.use_third_party_ua_parser,
            wait_for_country_lookup_init: data.wait_for_country_lookup_init,
            wait_for_user_agent_init: data.wait_for_user_agent_init,
            experimental_flags: data.experimental_flags,
        }
    }
}

#[no_mangle]
pub extern "C" fn statsig_options_create_from_data(json_data: *const c_char) -> u64 {
    let json_data = match c_char_to_string(json_data) {
        Some(data) => data,
        None => {
            log_e!(TAG, "Failed to convert c_char to string");
            return 0;
        }
    };

    let options_data = match serde_json::from_str::<StatsigOptionsData>(json_data.as_str()) {
        Ok(data) => data,
        Err(_) => {
            log_e!(TAG, "Failed to deserialize StatsigOptionsData");
            return 0;
        }
    };

    let options: StatsigOptions = options_data.into();

    InstanceRegistry::register(options).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigOptions");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_options_create(
    specs_url: *const c_char,
    log_event_url: *const c_char,
    specs_adapter_ref: u64,
    event_logging_adapter_ref: u64,
    environment: *const c_char,
    _event_logging_flush_interval_ms: c_int, // Deprecated
    event_logging_max_queue_size: c_int,
    specs_sync_interval_ms: c_int,
    output_log_level: *const c_char,
    disable_country_lookup: SafeOptBool,
    wait_for_country_lookup_init: SafeOptBool,
    wait_for_user_agent_init: SafeOptBool,
    enable_id_lists: SafeOptBool,
    enable_dcs_deltas: SafeOptBool,
    disable_network: SafeOptBool,
    id_lists_url: *const c_char,
    id_lists_sync_interval_ms: c_int,
    disable_all_logging: SafeOptBool,
    global_custom_fields: *const c_char,
    observability_client_ref: u64,
    data_store_ref: u64,
    init_timeout_ms: c_int,
    fallback_to_statsig_api: SafeOptBool,
    use_third_party_ua_parser: SafeOptBool,
    proxy_host: *const c_char,
    proxy_port: c_int,
    proxy_auth: *const c_char,
    proxy_protocol: *const c_char,
    persistent_storage_ref: u64,
) -> u64 {
    let specs_url = c_char_to_string(specs_url);
    let log_event_url = c_char_to_string(log_event_url);
    let id_lists_url = c_char_to_string(id_lists_url);
    let environment = c_char_to_string(environment);
    let event_logging_max_queue_size = c_int_to_u32(event_logging_max_queue_size);
    let specs_sync_interval_ms = c_int_to_u32(specs_sync_interval_ms);
    let id_lists_sync_interval_ms = c_int_to_u32(id_lists_sync_interval_ms);
    let global_custom_field_string = c_char_to_string(global_custom_fields);
    let global_custom_fields: Option<HashMap<String, DynamicValue>> =
        match global_custom_field_string {
            Some(s) => serde_json::from_str(s.as_str()).ok(),
            None => None,
        };
    let init_timeout_ms = c_int_to_u64(init_timeout_ms);

    let specs_adapter = try_get_specs_adapter(specs_adapter_ref);
    let event_logging_adapter = try_get_event_logging_adapter(event_logging_adapter_ref);
    let data_store = try_get_data_store(data_store_ref);
    let observability_client = try_get_observability_client(observability_client_ref);
    let persistent_storage = try_get_persistent_storage(persistent_storage_ref);

    let output_log_level =
        c_char_to_string(output_log_level).map(|level| LogLevel::from(level.as_str()));

    let proxy_config = create_proxy_config(
        c_char_to_string(proxy_host),
        if proxy_port > 0 {
            Some(proxy_port as u16)
        } else {
            None
        },
        c_char_to_string(proxy_auth),
        c_char_to_string(proxy_protocol),
    );

    InstanceRegistry::register(StatsigOptions {
        console_capture_options: None,
        specs_url,
        log_event_url,
        specs_adapter,
        event_logging_adapter,
        environment,
        event_logging_max_queue_size,
        specs_sync_interval_ms,
        output_log_level,
        disable_country_lookup: extract_opt_bool(disable_country_lookup),
        wait_for_country_lookup_init: extract_opt_bool(wait_for_country_lookup_init),
        wait_for_user_agent_init: extract_opt_bool(wait_for_user_agent_init),
        enable_id_lists: extract_opt_bool(enable_id_lists),
        enable_dcs_deltas: extract_opt_bool(enable_dcs_deltas),
        disable_network: extract_opt_bool(disable_network),
        id_lists_url,
        id_lists_sync_interval_ms,
        disable_all_logging: extract_opt_bool(disable_all_logging),
        global_custom_fields,
        data_store,
        observability_client,
        persistent_storage,
        init_timeout_ms,
        fallback_to_statsig_api: extract_opt_bool(fallback_to_statsig_api),
        use_third_party_ua_parser: extract_opt_bool(use_third_party_ua_parser),
        proxy_config,
        ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigOptions");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_options_release(options_ref: u64) {
    InstanceRegistry::remove(&options_ref);
}

fn try_get_specs_adapter(specs_adapter_ref: u64) -> Option<Arc<dyn SpecsAdapter>> {
    let raw = InstanceRegistry::get_raw(&specs_adapter_ref)?;

    if let Ok(adapter) = raw.clone().downcast::<StatsigLocalFileSpecsAdapter>() {
        return Some(adapter);
    }

    if let Ok(adapter) = raw.clone().downcast::<FunctionBasedSpecsAdapterC>() {
        return Some(adapter);
    }

    None
}

fn try_get_event_logging_adapter(
    event_logging_adapter_ref: u64,
) -> Option<Arc<dyn EventLoggingAdapter>> {
    let raw = InstanceRegistry::get_raw(&event_logging_adapter_ref)?;

    if let Ok(adapter) = raw
        .clone()
        .downcast::<StatsigLocalFileEventLoggingAdapter>()
    {
        return Some(adapter);
    }

    if let Ok(adapter) = raw.clone().downcast::<FunctionBasedEventLoggingAdapterC>() {
        return Some(adapter);
    }

    None
}

fn try_get_observability_client(
    observability_client_ref: u64,
) -> Option<Weak<dyn ObservabilityClient>> {
    let raw = InstanceRegistry::get_raw(&observability_client_ref)?;

    if let Ok(client) = raw.clone().downcast::<ObservabilityClientC>() {
        let weak = Arc::downgrade(&client);
        return Some(weak);
    }

    None
}

fn try_get_data_store(data_store_ref: u64) -> Option<Arc<dyn DataStoreTrait>> {
    let raw = InstanceRegistry::get_raw(&data_store_ref)?;

    if let Ok(data_store) = raw.clone().downcast::<DataStoreC>() {
        return Some(data_store);
    }

    None
}

fn try_get_persistent_storage(persistent_storage_ref: u64) -> Option<Arc<dyn PersistentStorage>> {
    let raw = InstanceRegistry::get_raw(&persistent_storage_ref)?;

    if let Ok(persistent_storage) = raw.clone().downcast::<PersistentStorageC>() {
        return Some(persistent_storage);
    }

    None
}

// This function is used to create a spec adapters config
// which is used to configure STATSIG FORWARD PROXY
#[allow(clippy::too_many_arguments)]
fn create_spec_adapters_config(
    spec_adapter_type: Option<String>,
    spec_adapter_url: Option<String>,
    spec_adapter_init_timeout_ms: Option<u64>,
    spec_adapter_authentication_mode: Option<String>,
    spec_adapter_ca_cert_path: Option<String>,
    spec_adapter_client_cert_path: Option<String>,
    spec_adapter_client_key_path: Option<String>,
    spec_adapter_domain_name: Option<String>,
) -> Option<Vec<SpecAdapterConfig>> {
    if let (Some(spec_adapter_type), Some(spec_adapter_init_timeout_ms)) =
        (spec_adapter_type, spec_adapter_init_timeout_ms)
    {
        return Some(vec![SpecAdapterConfig {
            adapter_type: spec_adapter_type.into(),
            specs_url: spec_adapter_url,
            init_timeout_ms: spec_adapter_init_timeout_ms,
            authentication_mode: spec_adapter_authentication_mode,
            ca_cert_path: spec_adapter_ca_cert_path,
            client_cert_path: spec_adapter_client_cert_path,
            client_key_path: spec_adapter_client_key_path,
            domain_name: spec_adapter_domain_name,
        }]);
    }

    None
}

fn create_proxy_config(
    proxy_host: Option<String>,
    proxy_port: Option<u16>,
    proxy_auth: Option<String>,
    proxy_protocol: Option<String>,
) -> Option<ProxyConfig> {
    // If no host is provided, no proxy config
    proxy_host.as_ref()?;

    Some(ProxyConfig {
        proxy_host,
        proxy_port,
        proxy_auth,
        proxy_protocol,
    })
}
