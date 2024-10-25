use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::statsig_user_c::StatsigUserRef;
use serde_json::json;
use sigstat::instance_store::{OPTIONS_INSTANCES, STATSIG_INSTANCES};
use sigstat::{log_e, log_w, unwrap_or_noop, Statsig};
use std::os::raw::c_char;
use std::slice;

#[repr(C)]
pub struct StatsigRef {
    pub pointer: usize,
}

impl StatsigRef {
    pub fn to_internal(&self) -> Option<&Statsig> {
        if self.pointer == 0 {
            log_w!("Failed to fetch Statsig. Reference has been released");
            return None;
        }

        Some(unsafe { &*(self.pointer as *mut Statsig) })
    }
}

#[no_mangle]
pub extern "C" fn statsig_create(
    sdk_key: *const c_char,
    options_ref: *const c_char,
) -> *const c_char {
    let sdk_key = c_char_to_string(sdk_key).unwrap();
    let options = match c_char_to_string(options_ref) {
        Some(id) => OPTIONS_INSTANCES.optional_get(Some(&id)),
        None => None,
    };

    let inst = Statsig::new(&sdk_key, options);

    let ref_id = STATSIG_INSTANCES.add(inst).unwrap_or_else(|| {
        log_e!("Failed to create Statsig");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_release(statsig_ref: *const c_char) {
    if let Some(id) = c_char_to_string(statsig_ref) {
        STATSIG_INSTANCES.release(id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_initialize(statsig_ref: *const c_char, callback: extern "C" fn()) {
    let ref_id = unwrap_or_noop!(c_char_to_string(statsig_ref));
    let statsig = unwrap_or_noop!(STATSIG_INSTANCES.get(&ref_id));

    statsig.initialize_with_callback(move || {
        callback();
    });
}

#[no_mangle]
pub extern "C" fn statsig_get_current_values(statsig_ref: StatsigRef) -> *const c_char {
    let statsig = match statsig_ref.to_internal() {
        Some(s) => s,
        None => return std::ptr::null(),
    };

    let values = statsig.get_current_values();
    let data = json!(values).to_string();
    string_to_c_char(data)
}

#[no_mangle]
pub extern "C" fn statsig_check_gate(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
    gate_name: *const c_char,
) -> bool {
    let statsig = match statsig_ref.to_internal() {
        Some(s) => s,
        None => return false,
    };

    let user = match user_ref.to_internal() {
        Some(u) => u,
        None => return false,
    };

    let gate_name = c_char_to_string(gate_name).unwrap();

    let bool_res = statsig.check_gate(user, &gate_name);
    return bool_res;
}

#[no_mangle]
pub extern "C" fn statsig_get_experiment(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
    experiment_name: *const c_char,
) -> *const c_char {
    let statsig = match statsig_ref.to_internal() {
        Some(s) => s,
        None => return std::ptr::null(),
    };
    let user = match user_ref.to_internal() {
        Some(u) => u,
        None => return std::ptr::null(),
    };
    let experiment_name = c_char_to_string(experiment_name).unwrap();

    let result = json!(statsig.get_experiment(user, &experiment_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_client_init_response(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
) -> *const c_char {
    let statsig = match statsig_ref.to_internal() {
        Some(s) => s,
        None => return std::ptr::null(),
    };
    let user = match user_ref.to_internal() {
        Some(u) => u,
        None => return std::ptr::null(),
    };

    let result = statsig.get_client_init_response(user);
    string_to_c_char(json!(result).to_string())
}

#[no_mangle]
pub extern "C" fn statsig_get_client_init_response_buffer(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    let statsig = statsig_ref.to_internal().unwrap();
    let user = user_ref.to_internal().unwrap();

    let result = statsig.get_client_init_response(user);
    let str = json!(result).to_string();
    let bytes = str.as_bytes();
    let length = bytes.len();
    let copy_size = std::cmp::min(length, buffer_size);

    unsafe {
        let slice = slice::from_raw_parts_mut(buffer as *mut u8, copy_size);
        slice.copy_from_slice(&bytes[..copy_size]);
    }

    copy_size
}
