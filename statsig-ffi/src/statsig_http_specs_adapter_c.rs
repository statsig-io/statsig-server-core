use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::get_instance_or_return_c;
use statsig_rust::{log_e, unwrap_or_return, InstanceRegistry, SpecsInfo, StatsigHttpSpecsAdapter};
use statsig_rust::{StatsigOptions, StatsigRuntime};
use std::os::raw::c_char;
use std::ptr::null;

const TAG: &str = "StatsigHttpSpecsAdapterC";

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_create(
    sdk_key: *const c_char,
    options_ref: *const c_char,
) -> *const c_char {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), null());

    let options_ref = c_char_to_string(options_ref);
    let options = InstanceRegistry::get_with_optional_id::<StatsigOptions>(options_ref.as_ref());

    let adapter = StatsigHttpSpecsAdapter::new(&sdk_key, options.as_ref().map(|o| o.as_ref()));

    let ref_id = InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigHttpSpecsAdapter");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_release(specs_adapter_ref: *const c_char) {
    if let Some(id) = c_char_to_string(specs_adapter_ref) {
        InstanceRegistry::remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_fetch_specs_from_network(
    specs_adapter_ref: *const c_char,
    current_specs_info: *const c_char,
) -> *const c_char {
    let specs_adapter =
        get_instance_or_return_c!(StatsigHttpSpecsAdapter, specs_adapter_ref, null());

    let statsig_rt = StatsigRuntime::get_runtime();

    let specs_info_str = match c_char_to_string(current_specs_info) {
        Some(s) => s,
        None => return null(),
    };

    let parsed_specs_info: SpecsInfo = match serde_json::from_str(&specs_info_str) {
        Ok(info) => info,
        Err(_) => return null(),
    };

    let result = statsig_rt.runtime_handle.block_on(async move {
        specs_adapter
            .fetch_specs_from_network(parsed_specs_info)
            .await
    });

    match result {
        Ok(data) => match String::from_utf8(data) {
            Ok(s) => string_to_c_char(s),
            Err(e) => {
                log_e!(TAG, "Failed to convert specs to string: {}", e);
                null()
            }
        },
        Err(e) => {
            log_e!(TAG, "Failed to fetch specs: {}", e);
            null()
        }
    }
}
