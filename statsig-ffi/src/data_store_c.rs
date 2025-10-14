use std::{collections::HashMap, ffi::c_char};

use async_trait::async_trait;
use serde_json::json;
use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    log_d, log_e, InstanceRegistry, StatsigErr, StatsigRuntime,
};

use crate::ffi_utils::{c_char_to_string, string_to_c_char};

const TAG: &str = "DataStoreC";

pub struct DataStoreC {
    pub initialize_fn: extern "C" fn(),
    pub shutdown_fn: extern "C" fn(),
    pub get_fn: extern "C" fn(key: *const c_char) -> *const c_char,
    pub set_fn: extern "C" fn(key: *const c_char, value: *const c_char, time: *const u64),
    pub support_polling_updates_for_fn: extern "C" fn(key: *const c_char) -> bool,
}

#[no_mangle]
pub extern "C" fn data_store_create(
    initialize_fn: extern "C" fn(),
    shutdown_fn: extern "C" fn(),
    get_fn: extern "C" fn(key: *const c_char) -> *const c_char,
    set_fn: extern "C" fn(key: *const c_char, value: *const c_char, time: *const u64),
    support_polling_updates_for_fn: extern "C" fn(key: *const c_char) -> bool,
) -> u64 {
    InstanceRegistry::register(DataStoreC {
        initialize_fn,
        shutdown_fn,
        get_fn,
        set_fn,
        support_polling_updates_for_fn,
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create DataStoreC");
        0
    })
}

#[no_mangle]
pub extern "C" fn data_store_release(data_store_ref: u64) {
    InstanceRegistry::remove(&data_store_ref);
}

#[no_mangle]
pub extern "C" fn __internal__test_data_store(
    data_store_ref: u64,
    path: *const c_char,
    value: *const c_char,
) -> *const c_char {
    let rt = match StatsigRuntime::get_runtime().get_handle() {
        Ok(rt) => rt,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return std::ptr::null();
        }
    };

    rt.block_on(async move {
        let path = match c_char_to_string(path) {
            Some(path) => path,
            None => {
                log_e!(TAG, "TEST Failed to convert path to string");
                return std::ptr::null();
            }
        };

        let value = match c_char_to_string(value) {
            Some(value) => value,
            None => {
                log_e!(TAG, "TEST Failed to convert value to string");
                return std::ptr::null();
            }
        };

        let store = match InstanceRegistry::get::<DataStoreC>(&data_store_ref) {
            Some(data_store) => data_store,
            None => {
                log_d!(TAG, "TEST DataStore reference not found");
                return std::ptr::null();
            }
        };

        if let Err(e) = store.initialize().await {
            log_e!(TAG, "TEST Failed to initialize data store: {}", e);
        }

        let get_result = match store.get(path.as_str()).await {
            Ok(result) => Some(result),
            Err(e) => {
                log_e!(TAG, "TEST Failed to get data from data store: {}", e);
                None
            }
        };

        if let Err(e) = store.set(path.as_str(), value.as_str(), Some(123)).await {
            log_e!(TAG, "TEST Failed to write to DataStore: {}", e);
        }

        let path = match path.as_str() {
            "/v2/download_config_specs" => Some(RequestPath::RulesetsV2),
            "/v1/download_config_specs" => Some(RequestPath::RulesetsV1),
            "/v1/get_id_lists" => Some(RequestPath::IDListsV1),
            "id_list" => Some(RequestPath::IDList),
            _ => {
                log_e!(TAG, "TEST Invalid request path: {path}");
                None
            }
        };

        let polling_result = match path {
            Some(path) => store.support_polling_updates_for(path).await,
            None => false,
        };

        if let Err(e) = store.shutdown().await {
            log_e!(TAG, "TEST Failed to shutdown data store: {}", e);
        }

        let result = serde_json::to_string(&HashMap::from([
            ("get_result".to_string(), json!(get_result)),
            ("polling_result".to_string(), json!(polling_result)),
        ]))
        .unwrap_or_default();

        string_to_c_char(result)
    })
}

#[async_trait]
impl DataStoreTrait for DataStoreC {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        (self.initialize_fn)();
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        (self.shutdown_fn)();
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        let key = string_to_c_char(key.to_string());
        if key.is_null() {
            return Err(StatsigErr::DataStoreFailure(
                "Failed to convert key to c_char".to_string(),
            ));
        }

        let raw_result = match c_char_to_string((self.get_fn)(key)) {
            Some(result) => result,
            None => {
                return Err(StatsigErr::DataStoreFailure(
                    "Failed to get result from data store".to_string(),
                ));
            }
        };

        serde_json::from_str::<DataStoreResponse>(&raw_result)
            .map_err(|e| StatsigErr::DataStoreFailure(e.to_string()))
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        let key = string_to_c_char(key.to_string());
        if key.is_null() {
            return Err(StatsigErr::DataStoreFailure(
                "Failed to convert key to c_char".to_string(),
            ));
        }

        let value = string_to_c_char(value.to_string());
        if value.is_null() {
            return Err(StatsigErr::DataStoreFailure(
                "Failed to convert value to c_char".to_string(),
            ));
        }

        let time: *const u64 = match &time {
            Some(v) => v as *const u64,
            None => std::ptr::null(),
        };

        (self.set_fn)(key, value, time);

        Ok(())
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        let path = string_to_c_char(path.to_string());
        if path.is_null() {
            log_e!(TAG, "Failed to convert DataStore RequestPath to c_char");
            return false;
        }

        (self.support_polling_updates_for_fn)(path)
    }
}
