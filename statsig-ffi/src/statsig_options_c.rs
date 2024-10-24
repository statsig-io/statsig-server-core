use std::os::raw::c_char;

use sigstat::{instance_store::OPTIONS_INSTANCES, log_e, log_w, StatsigOptions};

use crate::ffi_utils::{c_char_to_string, string_to_c_char};

#[repr(C)]
pub struct StatsigOptionsRef {
    pub pointer: usize,
}

impl StatsigOptionsRef {
    pub fn to_internal(&self) -> Option<&StatsigOptions> {
        if self.pointer == 0 {
            log_w!("Failed to fetch StatsigOptions. Reference has been released");
            return None;
        }

        Some(unsafe { &*(self.pointer as *mut StatsigOptions) })
    }
}

#[no_mangle]
pub extern "C" fn statsig_options_create() -> *const c_char {
    let ref_id = OPTIONS_INSTANCES
        .add(StatsigOptions {
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
