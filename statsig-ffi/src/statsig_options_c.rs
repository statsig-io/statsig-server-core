use std::{os::raw::c_char, os::raw::c_int, sync::Arc};

use crate::{
    data_store_c::DataStoreC,
    ffi_utils::{c_char_to_string, c_int_to_u32, c_int_to_u64, extract_opt_bool, SafeOptBool},
    function_based_event_logging_adapter_c::FunctionBasedEventLoggingAdapterC,
    function_based_specs_adapter_c::FunctionBasedSpecsAdapterC,
    observability_client_c::ObservabilityClientC,
};
use statsig_rust::{
    data_store_interface::DataStoreTrait, log_e, output_logger::LogLevel, DynamicValue,
    EventLoggingAdapter, InstanceRegistry, ObservabilityClient, SpecsAdapter,
    StatsigLocalFileEventLoggingAdapter, StatsigLocalFileSpecsAdapter, StatsigOptions,
};
use std::collections::HashMap;
use std::sync::Weak;

const TAG: &str = "StatsigOptionsC";

#[derive(Default, serde::Deserialize)]
pub struct StatsigOptionsData {
    specs_url: Option<String>,
    log_event_url: Option<String>,
    specs_adapter_ref: Option<u64>,
    event_logging_adapter_ref: Option<u64>,
    environment: Option<String>,
    event_logging_max_queue_size: Option<u32>,
    specs_sync_interval_ms: Option<u32>,
    output_log_level: Option<String>,
    disable_country_lookup: Option<bool>,
    disable_user_agent_parsing: Option<bool>,
    wait_for_country_lookup_init: Option<bool>,
    wait_for_user_agent_init: Option<bool>,
    enable_id_lists: Option<bool>,
    disable_network: Option<bool>,
    id_lists_url: Option<String>,
    id_lists_sync_interval_ms: Option<u32>,
    disable_all_logging: Option<bool>,
    global_custom_fields: Option<HashMap<String, DynamicValue>>,
    observability_client_ref: Option<u64>,
    data_store_ref: Option<u64>,
    init_timeout_ms: Option<u64>,
    fallback_to_statsig_api: Option<bool>,
}

impl From<StatsigOptionsData> for StatsigOptions {
    fn from(data: StatsigOptionsData) -> Self {
        Self {
            specs_url: data.specs_url,
            log_event_url: data.log_event_url,
            environment: data.environment,
            event_logging_max_queue_size: data.event_logging_max_queue_size,
            specs_sync_interval_ms: data.specs_sync_interval_ms,
            disable_country_lookup: data.disable_country_lookup,
            wait_for_country_lookup_init: data.wait_for_country_lookup_init,
            wait_for_user_agent_init: data.wait_for_user_agent_init,
            enable_id_lists: data.enable_id_lists,
            disable_network: data.disable_network,
            ..Default::default()
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

    let options: StatsigOptions =
        match serde_json::from_str::<StatsigOptionsData>(json_data.as_str()) {
            Ok(data) => data.into(),
            Err(_) => {
                log_e!(TAG, "Failed to deserialize StatsigOptionsData");
                return 0;
            }
        };

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

    let output_log_level =
        c_char_to_string(output_log_level).map(|level| LogLevel::from(level.as_str()));

    InstanceRegistry::register(StatsigOptions {
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
        disable_network: extract_opt_bool(disable_network),
        id_lists_url,
        id_lists_sync_interval_ms,
        disable_all_logging: extract_opt_bool(disable_all_logging),
        global_custom_fields,
        data_store,
        observability_client,
        init_timeout_ms,
        fallback_to_statsig_api: extract_opt_bool(fallback_to_statsig_api),
        use_third_party_ua_parser: extract_opt_bool(use_third_party_ua_parser),
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
