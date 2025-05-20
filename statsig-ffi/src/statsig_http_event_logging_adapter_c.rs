use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use statsig_rust::log_event_payload::LogEventRequest;
use statsig_rust::{log_e, unwrap_or_return, InstanceRegistry, StatsigRuntime};
use statsig_rust::{StatsigHttpEventLoggingAdapter, StatsigOptions};
use std::os::raw::c_char;
use std::ptr::null;
use std::sync::Arc;

const TAG: &str = "StatsigHttpEventLoggingAdapterC";

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_create(
    sdk_key: *const c_char,
    options_ref: u64,
) -> u64 {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), 0);
    let options = InstanceRegistry::get::<StatsigOptions>(&options_ref);

    let adapter = StatsigHttpEventLoggingAdapter::new(&sdk_key, options.as_ref().map(Arc::as_ref));

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigHttpSpecsAdapter");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_release(event_logging_adapter_ref: u64) {
    InstanceRegistry::remove(&event_logging_adapter_ref);
}

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_send_events(
    event_logging_adapter_ref: u64,
    request_json: *const c_char,
    callback: extern "C" fn(bool, *const c_char),
) {
    let handle_error = |msg: &str| {
        log_e!(TAG, "{}", msg);
        callback(false, string_to_c_char(msg.to_string()));
    };

    let event_logging_adapter =
        match InstanceRegistry::get::<StatsigHttpEventLoggingAdapter>(&event_logging_adapter_ref) {
            Some(adapter) => adapter,
            None => return handle_error("Failed to get event logging adapter instance"),
        };

    let request_json = match c_char_to_string(request_json) {
        Some(json) => json,
        None => return handle_error("Failed to convert request_json to string"),
    };

    let request = match serde_json::from_str::<LogEventRequest>(&request_json) {
        Ok(req) => req,
        Err(e) => return handle_error(&format!("Failed to parse request JSON: {}", e)),
    };

    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.block_on(async move {
        match event_logging_adapter.send_events_over_http(&request).await {
            Ok(_) => callback(true, null()),
            Err(e) => callback(false, string_to_c_char(e.to_string())),
        }
    });
}
