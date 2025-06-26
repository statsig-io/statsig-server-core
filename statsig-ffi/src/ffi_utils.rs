use serde_json::Value;
use statsig_rust::DynamicValue;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::{os::raw::c_char, os::raw::c_int};

// Do not use 'bool'. It's 1 byte, but its layout is not guaranteed stable
pub type SafeOptBool = c_int;

#[macro_export]
macro_rules! get_instance_or_noop_c {
    ($type:ty, $ref:expr) => {
        statsig_rust::get_instance_or_noop!($type, $ref)
    };
}

#[macro_export]
macro_rules! get_instance_or_return_c {
    ($type:ty, $ref:expr, $ret_value:expr) => {
        statsig_rust::get_instance_or_return!($type, $ref, $ret_value)
    };
}

pub fn c_char_to_string(c_str: *const c_char) -> Option<String> {
    if c_str.is_null() {
        return None;
    }

    let c_str = unsafe { CStr::from_ptr(c_str) };
    match c_str.to_str() {
        Ok(str_slice) => Some(str_slice.to_owned()),
        Err(_) => None,
    }
}

pub fn c_int_to_u32(c_num: c_int) -> Option<u32> {
    if c_num < 0 {
        return None;
    }

    Some(c_num as u32)
}

pub fn extract_opt_bool(c_bool: SafeOptBool) -> Option<bool> {
    if c_bool < 0 {
        return None;
    }

    Some(c_bool == 1)
}

pub fn string_to_c_char(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    // Retake ownership of the string and drop it.
    let _ = unsafe { CString::from_raw(s) };
}

pub fn parse_json_to_map(json_str: Option<String>) -> Option<HashMap<String, DynamicValue>> {
    if let Some(json_str) = json_str {
        match serde_json::from_str::<HashMap<String, Value>>(&json_str) {
            Ok(map) => {
                let dynamic_map = map
                    .into_iter()
                    .map(|(k, v)| (k, DynamicValue::from(v)))
                    .collect();
                Some(dynamic_map)
            }
            Err(_) => None,
        }
    } else {
        None
    }
}

pub fn parse_json_to_str_map(json_str: Option<String>) -> Option<HashMap<String, String>> {
    json_str.and_then(|s| serde_json::from_str(&s).ok())
}
