use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::get_instance_or_return_c;
use sigstat::instance_store::INST_STORE;
use sigstat::{log_e, unwrap_or_return, StatsigHttpSpecsAdapter};
use sigstat::{StatsigOptions, StatsigRuntime};
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
    let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_ref.as_ref());

    let mut specs_url = None;
    if let Some(options) = options {
        if let Some(url) = options.specs_url.clone() {
            specs_url = Some(url);
        }
    }

    let adapter = StatsigHttpSpecsAdapter::new(&sdk_key, specs_url.as_ref(), None);

    let ref_id = INST_STORE.add(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigHttpSpecsAdapter");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_release(specs_adapter_ref: *const c_char) {
    if let Some(id) = c_char_to_string(specs_adapter_ref) {
        INST_STORE.remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_fetch_specs_from_network(
    specs_adapter_ref: *const c_char,
    current_lcut: u64,
) -> *const c_char {
    let specs_adapter =
        get_instance_or_return_c!(StatsigHttpSpecsAdapter, specs_adapter_ref, null());

    let statsig_rt = StatsigRuntime::get_runtime();
    let result: Option<String> = statsig_rt.runtime_handle.block_on(async move {
        specs_adapter
            .fetch_specs_from_network(Some(current_lcut))
            .await
    });

    match result {
        Some(data) => string_to_c_char(data),
        None => null(),
    }
}
