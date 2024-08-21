use std::os::raw::c_char;
use std::slice;
use serde_json::json;
use statsig::Statsig;
use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::statsig_options_c::StatsigOptionsRef;
use crate::statsig_user_c::StatsigUserRef;

#[repr(C)]
pub struct StatsigRef {
    pub pointer: usize,
}

impl StatsigRef {
    pub fn to_internal(&self) -> Option<&Statsig> {
        if self.pointer == 0 {
            println!("Failed to fetch Statsig. Reference has been released");
            return None;
        }

        Some(unsafe { &*(self.pointer as *mut Statsig) })
    }
}


#[no_mangle]
pub extern "C" fn statsig_create(
    sdk_key: *const c_char,
    options_ref: StatsigOptionsRef,
) -> StatsigRef {
    let sdk_key = c_char_to_string(sdk_key).unwrap();
    let options = options_ref.to_internal();

    let inst = Statsig::new(&sdk_key, None);
    let pointer = Box::into_raw(Box::new(inst)) as usize;

    println!("Created Statsig {}", pointer);
    StatsigRef {
        pointer,
    }
}

#[no_mangle]
pub extern "C" fn statsig_release(statsig_ref: *mut StatsigRef) {
    let ref_obj = unsafe { &mut *statsig_ref };
    println!("Releasing Statsig {}", ref_obj.pointer);

    if ref_obj.pointer != 0 {
        unsafe { drop(Box::from_raw(ref_obj.pointer as *mut Statsig)) };
        ref_obj.pointer = 0;
        println!("Statsig released.");
    } else {
        println!("Warn: Statsig already released.");
    }
}

#[no_mangle]
pub extern "C" fn statsig_initialize(
    statsig_ref: StatsigRef,
    callback: extern "C" fn(),
) {
    println!("Statsig Init {}", statsig_ref.pointer);
    let statsig = statsig_ref.to_internal().unwrap();

    statsig.initialize_with_callback(move || {
        callback();
    });
}

#[no_mangle]
pub extern "C" fn statsig_get_current_values(
    statsig_ref: StatsigRef,
) -> *const c_char {
    //todo: handle unwrap
    let statsig = statsig_ref.to_internal().unwrap();

    let values = statsig.get_current_values();
    let data = json!(values).to_string();
    string_to_c_char(data)
}


#[no_mangle]
pub extern "C" fn statsig_check_gate(
    statsig_ptr: i64,
    user_ptr: i64
) -> bool {
    return false;
}

#[no_mangle]
pub extern "C" fn statsig_get_experiment(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
    experiment_name: *const c_char,
) -> *const c_char {
    //todo: handle unwrap
    let statsig = statsig_ref.to_internal().unwrap();
    let user = user_ref.to_internal().unwrap();
    let experiment_name = c_char_to_string(experiment_name).unwrap();

    let result = json!(statsig.get_experiment(user, &experiment_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_client_init_response(
    statsig_ref: StatsigRef,
    user_ref: StatsigUserRef,
) -> *const c_char {
    //todo: handle unwrap
    let statsig = statsig_ref.to_internal().unwrap();
    let user = user_ref.to_internal().unwrap();

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
