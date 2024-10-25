use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::statsig_user_c::StatsigUserRef;
use serde_json::json;
use sigstat::instance_store::{OPTIONS_INSTANCES, STATSIG_INSTANCES, USER_INSTANCES};
use sigstat::{log_e, log_w, unwrap_or_noop, unwrap_or_return, Statsig, StatsigUser};
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
pub extern "C" fn statsig_flush_events(statsig_ref: *const c_char, callback: extern "C" fn()) {
    let ref_id = unwrap_or_noop!(c_char_to_string(statsig_ref));
    let statsig = unwrap_or_noop!(STATSIG_INSTANCES.get(&ref_id));

    statsig.flush_events_with_callback(move || {
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
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    gate_name: *const c_char,
) -> bool {
    let statsig_ref = unwrap_or_return!(c_char_to_string(statsig_ref), false);
    let statsig = unwrap_or_return!(STATSIG_INSTANCES.get(&statsig_ref), false);

    let user_ref = unwrap_or_return!(c_char_to_string(user_ref), false);
    let user = unwrap_or_return!(USER_INSTANCES.get(&user_ref), false);

    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), false);

    statsig.check_gate(&user, &gate_name)
}

fn use_unwrapped_values<F, R>(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    opt_name: *const c_char,
    action: F,
) -> Option<R>
where
    F: FnOnce(&Statsig, &StatsigUser, &String) -> R,
{
    let statsig_ref = c_char_to_string(statsig_ref)?;
    let statsig = STATSIG_INSTANCES.get(&statsig_ref)?;

    let user_ref = c_char_to_string(user_ref)?;
    let user = USER_INSTANCES.get(&user_ref)?;

    let name = c_char_to_string(opt_name)?;
    Some(action(&statsig, &user, &name))
}

#[no_mangle]
pub extern "C" fn statsig_get_feature_gate(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    gate_name: *const c_char,
) -> *const c_char {
    let result = use_unwrapped_values(
        statsig_ref, user_ref, gate_name,
        |statsig, user, name| {
            json!(statsig.get_feature_gate(user, name)).to_string()
        });

    match result {
        Some(json_string) => string_to_c_char(json_string),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_dynamic_config(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    config_name: *const c_char,
) -> *const c_char {
    let result = use_unwrapped_values(
        statsig_ref, user_ref, config_name,
        |statsig, user, name| {
            json!(statsig.get_dynamic_config(user, name)).to_string()
        });

    match result {
        Some(json_string) => string_to_c_char(json_string),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_experiment(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    experiment_name: *const c_char,
) -> *const c_char {
    let result = use_unwrapped_values(
        statsig_ref, user_ref, experiment_name,
        |statsig, user, name| {
            json!(statsig.get_experiment(user, name)).to_string()
        });

    match result {
        Some(json_string) => string_to_c_char(json_string),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_layer(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    layer_name: *const c_char,
) -> *const c_char {
    let result = use_unwrapped_values(
        statsig_ref, user_ref, layer_name,
        |statsig, user, name| {
            json!(statsig.get_layer(user, name)).to_string()
        });

    match result {
        Some(json_string) => string_to_c_char(json_string),
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn statsig_log_layer_param_exposure(
    statsig_ref: *const c_char,
    layer_json: *const c_char,
    param_name: *const c_char,
) {
    let statsig_ref = unwrap_or_noop!(c_char_to_string(statsig_ref));
    let param_name = unwrap_or_noop!(c_char_to_string(param_name));
    let layer_json = unwrap_or_noop!(c_char_to_string(layer_json));

    let statsig = unwrap_or_noop!(STATSIG_INSTANCES.get(&statsig_ref));

    statsig.log_layer_param_exposure(layer_json, param_name);
}



#[no_mangle]
pub extern "C" fn statsig_get_client_init_response(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
) -> *const c_char {
    let statsig_ref = unwrap_or_return!(c_char_to_string(statsig_ref), std::ptr::null());
    let statsig = unwrap_or_return!(STATSIG_INSTANCES.get(&statsig_ref), std::ptr::null());

    let user_ref = unwrap_or_return!(c_char_to_string(user_ref), std::ptr::null());
    let user = unwrap_or_return!(USER_INSTANCES.get(&user_ref), std::ptr::null());

    let result = statsig.get_client_init_response(&user);
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
