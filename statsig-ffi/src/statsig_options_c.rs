use std::{os::raw::c_char, os::raw::c_int, sync::Arc};

use crate::{
    ffi_utils::{c_char_to_string, c_int_to_u32, string_to_c_char},
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
    specs_adapter_ref: *const c_char,
    event_logging_adapter_ref: *const c_char,
    environment: *const c_char,
    event_logging_flush_interval_ms: c_int,
    event_logging_max_queue_size: c_int,
    specs_sync_interval_ms: c_int,
    output_log_level: *const c_char,
) -> *const c_char {
    let specs_url = c_char_to_string(specs_url);
    let log_event_url = c_char_to_string(log_event_url);
    let environment = c_char_to_string(environment);
    let event_logging_flush_interval_ms = c_int_to_u32(event_logging_flush_interval_ms);
    let event_logging_max_queue_size = c_int_to_u32(event_logging_max_queue_size);
    let specs_sync_interval_ms = c_int_to_u32(specs_sync_interval_ms);

    let specs_adapter = try_get_specs_adapter(specs_adapter_ref);
    let event_logging_adapter = try_get_event_logging_adapter(event_logging_adapter_ref);

    let output_log_level =
        c_char_to_string(output_log_level).map(|level| LogLevel::from(level.as_str()));

    let ref_id = InstanceRegistry::register(StatsigOptions {
        specs_url,
        log_event_url,
        specs_adapter,
        event_logging_adapter,
        environment,
        event_logging_flush_interval_ms,
        event_logging_max_queue_size,
        specs_sync_interval_ms,
        output_log_level,
        ..StatsigOptions::new()
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigOptions");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_options_release(options_ref: *const c_char) {
    if let Some(id) = c_char_to_string(options_ref) {
        InstanceRegistry::remove(&id);
    }
}

fn try_get_specs_adapter(specs_adapter_ref: *const c_char) -> Option<Arc<dyn SpecsAdapter>> {
    let specs_adapter_ref = c_char_to_string(specs_adapter_ref)?;
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
    event_logging_adapter_ref: *const c_char,
) -> Option<Arc<dyn EventLoggingAdapter>> {
    let event_logging_adapter_ref = c_char_to_string(event_logging_adapter_ref)?;
    let raw = InstanceRegistry::get_raw(&event_logging_adapter_ref)?;

    if let Ok(adapter) = raw
        .clone()
        .downcast::<StatsigLocalFileEventLoggingAdapter>()
    {
        return Some(adapter);
    }

    None
}
