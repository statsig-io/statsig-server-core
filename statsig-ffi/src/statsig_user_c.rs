use statsig::{StatsigUser};
use std::os::raw::c_char;
use statsig::statsig_user::StatsigUserBuilder;
use crate::ffi_utils::{c_char_to_string, parse_json_to_map};

#[repr(C)]
pub struct StatsigUserRef {
    pub pointer: usize,
}

impl StatsigUserRef {
    pub fn to_internal(&self) -> Option<&StatsigUser> {
        if self.pointer == 0 {
            println!("Failed to fetch StatsigUser. Reference has been released");
            return None;
        }

        Some(unsafe { &*(self.pointer as *mut StatsigUser) })
    }
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
) -> StatsigUserRef {
    // Convert C strings to Rust Options
    let user_id = c_char_to_string(user_id);
    let custom_ids = parse_json_to_map(c_char_to_string(custom_ids_json));
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
        None => {
            StatsigUserBuilder::new_with_user_id(user_id.unwrap_or_default())
                .custom_ids(None)
        }
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

    // Build the StatsigUser and convert it to a raw pointer
    let user = builder.build();
    let pointer = Box::into_raw(Box::new(user)) as usize;

    StatsigUserRef { pointer }
}

#[no_mangle]
pub extern "C" fn statsig_user_release(user_ref: *mut StatsigUserRef) {
    let ref_obj = unsafe { &mut *user_ref };
    // println!("Releasing StatsigUser {}", ref_obj.pointer);

    if ref_obj.pointer != 0 {
        unsafe { drop(Box::from_raw(ref_obj.pointer as *mut StatsigUser)) };
        ref_obj.pointer = 0;
        // println!("StatsigUser released.");
    } else {
        println!("Warn: StatsigUser already released.");
    }
}
