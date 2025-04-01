use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::get_instance_or_noop_c;
use statsig_rust::{
    log_e, unwrap_or_return, InstanceRegistry, StatsigLocalFileSpecsAdapter, StatsigRuntime,
};
use std::os::raw::c_char;

const TAG: &str = "StatsigLocalFileSpecsAdapterC";

#[no_mangle]
pub extern "C" fn statsig_local_file_specs_adapter_create(
    sdk_key: *const c_char,
    output_directory: *const c_char,
    specs_url: *const c_char,
    fallback_to_statsig_api: bool,
    disable_network: bool,
) -> *const c_char {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), std::ptr::null());
    let output_directory = unwrap_or_return!(c_char_to_string(output_directory), std::ptr::null());
    let specs_url = c_char_to_string(specs_url).map(Some).unwrap_or_default();

    let adapter = StatsigLocalFileSpecsAdapter::new(
        &sdk_key,
        &output_directory,
        specs_url,
        fallback_to_statsig_api,
        disable_network,
    );

    let ref_id = InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigLocalFileSpecsAdapter");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_local_file_specs_adapter_release(specs_adapter_ref: *const c_char) {
    if let Some(id) = c_char_to_string(specs_adapter_ref) {
        InstanceRegistry::remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_local_file_specs_adapter_fetch_and_write_to_file(
    specs_adapter_ref: *const c_char,
) {
    let specs_adapter = get_instance_or_noop_c!(StatsigLocalFileSpecsAdapter, specs_adapter_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    let result = statsig_rt
        .runtime_handle
        .block_on(async move { specs_adapter.fetch_and_write_to_file().await });

    if let Err(e) = result {
        log_e!(TAG, "Failed to write to file: {}", e);
    }
}
