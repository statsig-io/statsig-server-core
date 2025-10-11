use crate::ffi_utils::{c_char_to_string, parse_json_to_map, parse_json_to_str_map};
use statsig_rust::{log_e, InstanceRegistry, StatsigUser};
use statsig_rust::{StatsigUserBuilder, StatsigUserData};
use std::os::raw::c_char;
use std::sync::Arc;

const TAG: &str = "StatsigUserC";

#[no_mangle]
pub extern "C" fn statsig_user_create_from_data(json_data: *const c_char) -> u64 {
    let json_data = match c_char_to_string(json_data) {
        Some(data) => data,
        None => {
            log_e!(TAG, "Failed to convert c_char to string");
            return 0;
        }
    };

    let user_data = match serde_json::from_str::<StatsigUserData>(json_data.as_str()) {
        Ok(data) => data,
        Err(_) => {
            log_e!(TAG, "Failed to deserialize StatsigUserData");
            return 0;
        }
    };

    let user = StatsigUser {
        data: Arc::new(user_data),
    };

    InstanceRegistry::register(user).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigUser");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_user_create(
    user_id: *const c_char,
    custom_ids_json: *const c_char,
    email: *const c_char,
    ip: *const c_char,
    user_agent: *const c_char,
    country: *const c_char,
    locale: *const c_char,
    app_version: *const c_char,
    custom_json: *const c_char,
    private_attributes_json: *const c_char,
) -> u64 {
    let user_id = c_char_to_string(user_id);
    let custom_ids = parse_json_to_str_map(c_char_to_string(custom_ids_json));
    let email = c_char_to_string(email);
    let ip = c_char_to_string(ip);
    let user_agent = c_char_to_string(user_agent);
    let country = c_char_to_string(country);
    let locale = c_char_to_string(locale);
    let app_version = c_char_to_string(app_version);
    let custom = parse_json_to_map(c_char_to_string(custom_json));
    let private_attributes = parse_json_to_map(c_char_to_string(private_attributes_json));

    let mut builder = match custom_ids {
        Some(custom_ids) => StatsigUserBuilder::new_with_custom_ids(custom_ids).user_id(user_id),
        None => StatsigUserBuilder::new_with_user_id(user_id.unwrap_or_default()),
    };

    builder = builder
        .email(email)
        .ip(ip)
        .user_agent(user_agent)
        .country(country)
        .locale(locale)
        .app_version(app_version)
        .custom(custom)
        .private_attributes(private_attributes);

    let user = builder.build();

    InstanceRegistry::register(user).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create StatsigUser");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_user_release(user_ref: u64) {
    InstanceRegistry::remove(&user_ref);
}
