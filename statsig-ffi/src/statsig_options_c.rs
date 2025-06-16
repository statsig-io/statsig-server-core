use std::{os::raw::c_char, os::raw::c_int, sync::Arc};

use crate::{
    ffi_utils::{c_char_to_string, c_int_to_u32, extract_opt_bool, SafeOptBool},
    function_based_event_logging_adapter_c::FunctionBasedEventLoggingAdapterC,
    function_based_specs_adapter_c::FunctionBasedSpecsAdapterC,
};
use statsig_rust::{
    log_e, output_logger::LogLevel, EventLoggingAdapter, InstanceRegistry, SpecsAdapter,
    StatsigLocalFileEventLoggingAdapter, StatsigLocalFileSpecsAdapter, StatsigOptions,
};

const TAG: &str = "StatsigOptionsC";

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
    disable_user_agent_parsing: SafeOptBool,
    wait_for_country_lookup_init: SafeOptBool,
    wait_for_user_agent_init: SafeOptBool,
    enable_id_lists: SafeOptBool,
    id_lists_url: *const c_char,
    id_lists_sync_interval_ms: c_int,
) -> u64 {
    let specs_url = c_char_to_string(specs_url);
    let log_event_url = c_char_to_string(log_event_url);
    let id_lists_url = c_char_to_string(id_lists_url);
    let environment = c_char_to_string(environment);
    let event_logging_max_queue_size = c_int_to_u32(event_logging_max_queue_size);
    let specs_sync_interval_ms = c_int_to_u32(specs_sync_interval_ms);
    let id_lists_sync_interval_ms = c_int_to_u32(id_lists_sync_interval_ms);

    let specs_adapter = try_get_specs_adapter(specs_adapter_ref);
    let event_logging_adapter = try_get_event_logging_adapter(event_logging_adapter_ref);

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
        disable_user_agent_parsing: extract_opt_bool(disable_user_agent_parsing),
        wait_for_country_lookup_init: extract_opt_bool(wait_for_country_lookup_init),
        wait_for_user_agent_init: extract_opt_bool(wait_for_user_agent_init),
        enable_id_lists: extract_opt_bool(enable_id_lists),
        id_lists_url,
        id_lists_sync_interval_ms,
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
