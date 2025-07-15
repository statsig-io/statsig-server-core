use crate::ffi_utils::c_char_to_string;
use crate::get_instance_or_noop_c;
use statsig_rust::{
    log_e, unwrap_or_return, InstanceRegistry, StatsigLocalFileEventLoggingAdapter, StatsigRuntime,
};
use std::os::raw::c_char;

const TAG: &str = "StatsigLocalFileEventLoggingAdapterC";

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_create(
    sdk_key: *const c_char,
    output_directory: *const c_char,
    log_event_url: *const c_char,
    disable_network: bool,
) -> u64 {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), 0);
    let output_directory = unwrap_or_return!(c_char_to_string(output_directory), 0);
    let log_event_url = c_char_to_string(log_event_url)
        .map(Some)
        .unwrap_or_default();

    let adapter = StatsigLocalFileEventLoggingAdapter::new(
        &sdk_key,
        &output_directory,
        log_event_url,
        disable_network,
    );

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigLocalFileSpecsAdapter");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_release(event_logging_adapter_ref: u64) {
    InstanceRegistry::remove(&event_logging_adapter_ref);
}

#[no_mangle]
pub extern "C" fn statsig_local_file_event_logging_adapter_send_pending_events(adapter_ref: u64) {
    let adapter = get_instance_or_noop_c!(StatsigLocalFileEventLoggingAdapter, &adapter_ref);

    let rt_handle = match StatsigRuntime::get_runtime().get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return;
        }
    };

    let result = rt_handle.block_on(async move { adapter.send_pending_events().await });
    if let Err(e) = result {
        log_e!(TAG, "Failed to send events: {}", e);
    }
}
