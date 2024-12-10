use std::{os::raw::c_char, sync::Arc};

use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use sigstat::{
    instance_store::INST_STORE, log_e, EventLoggingAdapter, SpecsAdapter,
    StatsigLocalFileEventLoggingAdapter, StatsigLocalFileSpecsAdapter, StatsigOptions,
};

const TAG: &str = "StatsigOptionsC";

#[no_mangle]
pub extern "C" fn statsig_options_create(
    specs_url: *const c_char,
    log_event_url: *const c_char,
    specs_adapter_ref: *const c_char,
    event_logging_adapter_ref: *const c_char,
) -> *const c_char {
    let specs_url = c_char_to_string(specs_url);
    let log_event_url = c_char_to_string(log_event_url);

    let specs_adapter: Option<Arc<dyn SpecsAdapter>> = match c_char_to_string(specs_adapter_ref) {
        Some(specs_adapter_ref) => INST_STORE
            .get::<StatsigLocalFileSpecsAdapter>(&specs_adapter_ref)
            .map(|adapter| adapter as Arc<dyn SpecsAdapter>),
        None => None,
    };

    let event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>> =
        match c_char_to_string(event_logging_adapter_ref) {
            Some(event_logging_adapter_ref) => INST_STORE
                .get::<StatsigLocalFileEventLoggingAdapter>(&event_logging_adapter_ref)
                .map(|adapter| adapter as Arc<dyn EventLoggingAdapter>),
            None => None,
        };

    let ref_id = INST_STORE
        .add(StatsigOptions {
            specs_url,
            log_event_url,
            specs_adapter,
            event_logging_adapter,
            ..StatsigOptions::new()
        })
        .unwrap_or_else(|| {
            log_e!(TAG, "Failed to create StatsigOptions");
            "".to_string()
        });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_options_release(options_ref: *const c_char) {
    if let Some(id) = c_char_to_string(options_ref) {
        INST_STORE.remove(&id);
    }
}
