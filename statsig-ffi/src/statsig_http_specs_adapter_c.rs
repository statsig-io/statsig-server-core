use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::get_instance_or_return_c;
use sigstat::instance_store::INST_STORE;
use sigstat::StatsigOptions;
use sigstat::{log_e, unwrap_or_return, StatsigHttpSpecsAdapter};
use std::os::raw::c_char;
use std::ptr::null;

#[no_mangle]
pub extern "C" fn statsig_http_specs_adapter_create(
    sdk_key: *const c_char,
    options_ref: *const c_char,
) -> *const c_char {
    let sdk_key = unwrap_or_return!(c_char_to_string(sdk_key), std::ptr::null());

    let options_ref = c_char_to_string(options_ref);
    let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_ref.as_ref());

    let mut specs_url = None;
    if let Some(options) = options {
        if let Some(url) = options.specs_url.clone() {
            specs_url = Some(url);
        }
    }

    let adapter = StatsigHttpSpecsAdapter::new(&sdk_key, specs_url.as_ref(), 0, None);

    let ref_id = INST_STORE.add(adapter).unwrap_or_else(|| {
        log_e!("Failed to create StatsigHttpSpecsAdapter");
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

    let result = match specs_adapter.fetch_specs_from_network(Some(current_lcut)) {
        Some(data) => data,
        None => {
            log_e!("Failed to fetch specs from network");
            return null();
        }
    };

    string_to_c_char(result)
}
