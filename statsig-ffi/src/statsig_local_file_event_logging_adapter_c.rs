use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use sigstat::instance_store::INST_STORE;
use sigstat::{log_e, unwrap_or_return, StatsigLocalFileEventLoggingAdapter, StatsigRuntime};
use std::os::raw::c_char;
use crate::get_instance_or_noop_c;

const TAG: &str = "StatsigLocalFileEventLoggingAdapterC";

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_create(
    sdk_key: *const c_char,
    output_directory: *const c_char,
    log_event_url: *const c_char
) -> *const c_char {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), std::ptr::null());
    let output_directory = unwrap_or_return!(c_char_to_string(output_directory), std::ptr::null());
    let log_event_url = c_char_to_string(log_event_url).map(|u| Some(u)).unwrap_or_default();

    let adapter = StatsigLocalFileEventLoggingAdapter::new(&sdk_key, &output_directory, log_event_url);

    let ref_id = INST_STORE.add(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigLocalFileSpecsAdapter");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_release(
    event_logging_adapter_ref: *const c_char,
) {
    if let Some(id) = c_char_to_string(event_logging_adapter_ref) {
        INST_STORE.remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_send_pending_events(adapter_ref: *const c_char) {
    let adapter =
        get_instance_or_noop_c!(StatsigLocalFileEventLoggingAdapter, adapter_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    let result = statsig_rt.runtime_handle.block_on(async move {
        adapter.send_pending_events().await
    });

    if let Err(e) = result {
        log_e!(TAG, "Failed to send events: {}", e);
    }
}
