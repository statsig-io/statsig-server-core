use crate::ffi_utils::c_char_to_string;
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
) -> u64 {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), 0);
    let output_directory = unwrap_or_return!(c_char_to_string(output_directory), 0);
    let specs_url = c_char_to_string(specs_url).map(Some).unwrap_or_default();

    let adapter = StatsigLocalFileSpecsAdapter::new(
        &sdk_key,
        &output_directory,
        specs_url,
        fallback_to_statsig_api,
        disable_network,
    );

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigLocalFileSpecsAdapter");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_local_file_specs_adapter_release(specs_adapter_ref: u64) {
    InstanceRegistry::remove(&specs_adapter_ref);
}

#[no_mangle]
pub extern "C" fn statsig_local_file_specs_adapter_fetch_and_write_to_file(specs_adapter_ref: u64) {
    let specs_adapter = get_instance_or_noop_c!(StatsigLocalFileSpecsAdapter, &specs_adapter_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    let result = statsig_rt
        .runtime_handle
        .block_on(async move { specs_adapter.fetch_and_write_to_file().await });

    if let Err(e) = result {
        log_e!(TAG, "Failed to write to file: {}", e);
    }
}
