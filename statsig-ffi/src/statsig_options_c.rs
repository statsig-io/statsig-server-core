use std::os::raw::c_char;

use sigstat::{instance_store::OPTIONS_INSTANCES, log_e, StatsigOptions};
use crate::ffi_utils::{c_char_to_string, string_to_c_char};

#[no_mangle]
pub extern "C" fn statsig_options_create(
    specs_url: *const c_char
) -> *const c_char {
    let specs_url = c_char_to_string(specs_url);

    let ref_id = OPTIONS_INSTANCES
        .add(StatsigOptions {
            specs_url,
            ..StatsigOptions::new()
        })
        .unwrap_or_else(|| {
            log_e!("Failed to create StatsigOptions");
            "".to_string()
        });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_options_release(options_ref: *const c_char) {
    if let Some(id) = c_char_to_string(options_ref) {
        OPTIONS_INSTANCES.release(id);
    }
}
