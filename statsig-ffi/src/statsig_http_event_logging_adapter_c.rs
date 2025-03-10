use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use statsig_rust::instance_store::INST_STORE;
use statsig_rust::log_event_payload::LogEventRequest;
use statsig_rust::{log_e, unwrap_or_return, StatsigRuntime};
use statsig_rust::{StatsigHttpEventLoggingAdapter, StatsigOptions};
use std::os::raw::c_char;
use std::ptr::null;

const TAG: &str = "StatsigHttpEventLoggingAdapterC";

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_create(
    sdk_key: *const c_char,
    options_ref: *const c_char,
) -> *const c_char {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), null());

    let options_ref = c_char_to_string(options_ref);
    let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_ref.as_ref());

    let mut log_event_url = None;
    if let Some(options) = options {
        log_event_url = options.log_event_url.clone();
    }

    let adapter = StatsigHttpEventLoggingAdapter::new(&sdk_key, log_event_url.as_ref());

    let ref_id = INST_STORE.add(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigHttpSpecsAdapter");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_release(
    event_logging_adapter_ref: *const c_char,
) {
    if let Some(id) = c_char_to_string(event_logging_adapter_ref) {
        INST_STORE.remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_http_event_logging_adapter_send_events(
    event_logging_adapter_ref: *const c_char,
    request_json: *const c_char,
    callback: extern "C" fn(bool, *const c_char),
) {
    let handle_error = |msg: &str| {
        log_e!(TAG, "{}", msg);
        callback(false, string_to_c_char(msg.to_string()));
    };

    let event_logging_adapter_ref = match c_char_to_string(event_logging_adapter_ref) {
        Some(ref_str) => ref_str,
        None => return handle_error("Failed to convert event_logging_adapter_ref to string"),
    };

    let event_logging_adapter =
        match INST_STORE.get::<StatsigHttpEventLoggingAdapter>(&event_logging_adapter_ref) {
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
