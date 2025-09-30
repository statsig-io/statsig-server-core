use std::{
    ffi::{c_char, c_ulong},
    ptr::null_mut,
    sync::Arc,
};

use serde_json::json;
use statsig_rust::{
    log_e, networking::ResponseData, unwrap_or_else, InstanceRegistry, SpecsSource, SpecsUpdate,
    SpecsUpdateListener,
};

use crate::{
    ffi_utils::{c_char_to_string, string_to_c_char},
    get_instance_or_noop_c, get_instance_or_return_c,
};

const TAG: &str = "SpecsUpdateListenerC";

pub struct SpecsUpdateListenerC {
    pub inner: Arc<dyn SpecsUpdateListener>,
}

#[no_mangle]
pub extern "C" fn specs_update_listener_release(listener_ref: u64) {
    InstanceRegistry::remove(&listener_ref);
}

// todo: Windows builders complain if 'received_at.into()' is not present
//  Likely they are running an older version of rustc. This can be removed once the version is updated.
#[allow(clippy::useless_conversion)]
#[no_mangle]
pub extern "C" fn specs_update_listener_did_receive_specs_update(
    listener_ref: u64,
    data: *const c_char,
    source: *const c_char,
    received_at: c_ulong,
) {
    let listener = get_instance_or_noop_c!(SpecsUpdateListenerC, &listener_ref);

    let data = unwrap_or_else!(c_char_to_string(data), {
        log_e!(TAG, "Failed to convert 'data' to Rust string");
        return;
    });

    let source = unwrap_or_else!(c_char_to_string(source), {
        log_e!(TAG, "Failed to convert 'source' to Rust string");
        return;
    });

    let update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()), // todo: update the c function to take a slice
        source: SpecsSource::new_from_string(&source),
        received_at: received_at.into(),
        source_api: None,
    };

    if let Err(e) = listener.inner.did_receive_specs_update(update) {
        log_e!(TAG, "Failed to call did_receive_specs_update: {}", e);
    }
}

#[no_mangle]
pub extern "C" fn specs_update_listener_get_current_specs_info(listener_ref: u64) -> *mut c_char {
    let listener = get_instance_or_return_c!(SpecsUpdateListenerC, &listener_ref, null_mut());

    let info = listener.inner.get_current_specs_info();
    let result = json!(info).to_string();
    string_to_c_char(result)
}
