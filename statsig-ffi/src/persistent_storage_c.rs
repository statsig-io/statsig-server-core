use std::ffi::c_char;

use async_trait::async_trait;
use serde::Serialize;
use statsig_rust::{
    log_d, log_e, unwrap_or_return, InstanceRegistry, PersistentStorage, StickyValues,
    UserPersistedValues,
};

use crate::ffi_utils::{c_char_to_string, string_to_c_char};

const TAG: &str = "PersistentStorageC";

#[derive(Serialize)]
struct PersistentStorageArgs<'a> {
    key: &'a str,
    config_name: &'a str,
    data: &'a Option<StickyValues>,
}

pub struct PersistentStorageC {
    pub load_fn: extern "C" fn(key: *const c_char) -> *mut c_char,
    pub save_fn: extern "C" fn(args: *const c_char),
    pub delete_fn: extern "C" fn(args: *const c_char),
}

// -------------------------------------------------------------------- [ Trait Impl ]

#[async_trait]
impl PersistentStorage for PersistentStorageC {
    fn load(&self, key: String) -> Option<UserPersistedValues> {
        let key_char = string_to_c_char(key);
        let result = (self.load_fn)(key_char);
        let result_str = c_char_to_string(result).filter(|s| !s.is_empty())?;

        match serde_json::from_str::<UserPersistedValues>(&result_str) {
            Ok(result_json) => Some(result_json),
            Err(e) => {
                log_e!(TAG, "Failed to deserialize UserPersistedValues: {}", e);
                None
            }
        }
    }

    fn save(&self, key: &str, config_name: &str, data: StickyValues) {
        let args = parcel_args("save", key, config_name, &Some(data));
        if args.is_null() {
            return;
        }

        (self.save_fn)(args);
    }

    fn delete(&self, _key: &str, _config_name: &str) {
        let args = parcel_args("delete", _key, _config_name, &None);
        if args.is_null() {
            return;
        }

        (self.delete_fn)(args);
    }
}

// -------------------------------------------------------------------- [ C Extern ]

#[no_mangle]
pub extern "C" fn persistent_storage_create(
    load_fn: extern "C" fn(key: *const c_char) -> *mut c_char,
    save_fn: extern "C" fn(args: *const c_char),
    delete_fn: extern "C" fn(args: *const c_char),
) -> u64 {
    InstanceRegistry::register(PersistentStorageC {
        load_fn,
        save_fn,
        delete_fn,
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create PersistentStorageC");
        0
    })
}

#[no_mangle]
pub extern "C" fn persistent_storage_release(storage_ref: u64) {
    InstanceRegistry::remove(&storage_ref);
}

#[no_mangle]
pub extern "C" fn __internal__test_persistent_storage(
    storage_ref: u64,
    action: *const c_char,
    key: *const c_char,
    config_name: *const c_char,
    data: *const c_char,
) -> *mut c_char {
    let storage = match InstanceRegistry::get::<PersistentStorageC>(&storage_ref) {
        Some(persistent_storage) => persistent_storage,
        None => {
            log_d!(TAG, "TEST PersistentStorage reference not found");
            return std::ptr::null_mut();
        }
    };

    let action = unwrap_or_return!(c_char_to_string(action), std::ptr::null_mut());
    let key = unwrap_or_return!(c_char_to_string(key), std::ptr::null_mut());
    let config_name = c_char_to_string(config_name).filter(|config_name| !config_name.is_empty());
    let data = c_char_to_string(data)
        .filter(|data| !data.is_empty())
        .map(|data| serde_json::from_str::<StickyValues>(&data).unwrap());

    match action.as_str() {
        "load" => {
            let result = storage.load(key);
            return string_to_c_char(serde_json::to_string(&result).unwrap());
        }
        "save" => {
            storage.save(key.as_str(), config_name.unwrap().as_str(), data.unwrap());
        }
        "delete" => {
            storage.delete(key.as_str(), config_name.unwrap().as_str());
        }
        _ => panic!("Invalid action: {action}"),
    };

    std::ptr::null_mut()
}

fn parcel_args(
    action: &'static str,
    key: &str,
    config_name: &str,
    data: &Option<StickyValues>,
) -> *mut c_char {
    let args = PersistentStorageArgs {
        key,
        config_name,
        data,
    };

    let json = match serde_json::to_string(&args) {
        Ok(json) => json,
        Err(e) => {
            log_e!(
                TAG,
                "Failed to serialize PersistentStorageArgs for '{action}': {}",
                e
            );
            return std::ptr::null_mut();
        }
    };

    string_to_c_char(json)
}
