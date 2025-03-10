use std::ffi::c_char;

use statsig_rust::{statsig_metadata::StatsigMetadata, unwrap_or_noop};

use crate::ffi_utils::c_char_to_string;

#[no_mangle]
pub extern "C" fn statsig_metadata_update_values(
    sdk_type: *const c_char,
    os: *const c_char,
    arch: *const c_char,
    language_version: *const c_char,
) {
    let sdk_type = unwrap_or_noop!(c_char_to_string(sdk_type));
    let os = unwrap_or_noop!(c_char_to_string(os));
    let arch = unwrap_or_noop!(c_char_to_string(arch));
    let language_version = unwrap_or_noop!(c_char_to_string(language_version));
    StatsigMetadata::update_values(sdk_type, os, arch, language_version);
}
