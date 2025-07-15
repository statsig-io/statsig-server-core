use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::get_instance_or_return_c;
use statsig_rust::{log_e, unwrap_or_return, InstanceRegistry, SpecsInfo, StatsigHttpSpecsAdapter};
use statsig_rust::{StatsigOptions, StatsigRuntime};
use std::os::raw::c_char;
use std::ptr::null_mut;

const TAG: &str = "StatsigHttpSpecsAdapterC";

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_create(
    sdk_key: *const c_char,
    options_ref: u64,
) -> u64 {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), 0);

    let options = InstanceRegistry::get::<StatsigOptions>(&options_ref);

    let adapter =
        StatsigHttpSpecsAdapter::new(&sdk_key, options.as_ref().map(|o| o.as_ref()), None);

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigHttpSpecsAdapter");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_release(specs_adapter_ref: u64) {
    InstanceRegistry::remove(&specs_adapter_ref);
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_fetch_specs_from_network(
    specs_adapter_ref: u64,
    current_specs_info: *const c_char,
) -> *mut c_char {
    let specs_adapter =
        get_instance_or_return_c!(StatsigHttpSpecsAdapter, &specs_adapter_ref, null_mut());

    let rt_handle = match StatsigRuntime::get_runtime().get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return null_mut();
        }
    };

    let specs_info_str = match c_char_to_string(current_specs_info) {
        Some(s) => s,
        None => return null_mut(),
    };

    let parsed_specs_info: SpecsInfo = match serde_json::from_str(&specs_info_str) {
        Ok(info) => info,
        Err(_) => return null_mut(),
    };

    let result = rt_handle.block_on(async move {
        specs_adapter
            .fetch_specs_from_network(parsed_specs_info)
            .await
    });
    match result {
        Ok(response) => match String::from_utf8(response.data) {
            Ok(s) => string_to_c_char(s),
            Err(e) => {
                log_e!(TAG, "Failed to convert specs to string: {}", e);
                null_mut()
            }
        },
        Err(e) => {
            log_e!(TAG, "Failed to fetch specs: {}", e);
            null_mut()
        }
    }
}
