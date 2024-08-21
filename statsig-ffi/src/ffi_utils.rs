use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

pub fn string_to_c_char(s: String) -> *const c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

pub fn parse_json_to_map(json_str: Option<String>) -> Option<HashMap<String, String>> {
    json_str.and_then(|s| serde_json::from_str(&s).ok())
}