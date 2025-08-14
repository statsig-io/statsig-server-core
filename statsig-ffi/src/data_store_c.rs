use crate::ffi_utils::string_to_c_char;
use async_trait::async_trait;

use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    log_e, InstanceRegistry, StatsigErr,
};

use std::ffi::{c_char, CStr, CString};
use std::sync::Mutex;

const TAG: &str = "DataStoreAdapterC";
pub struct DataStoreC {
    pub data_store_ref: Mutex<Option<u64>>,
    pub initialize_fn: extern "C" fn(data_store_ref: u64),
    pub shutdown_fn: extern "C" fn(data_store_ref: u64),
    pub get_fn: extern "C" fn(data_store_ref: u64, key: *const c_char) -> *const c_char,
    pub set_fn: extern "C" fn(
        data_store_ref: u64,
        key: *const c_char,
        value: *const c_char,
        time: *const u64,
    ),
    pub support_polling_updates_for_fn:
        extern "C" fn(data_store_ref: u64, key: *const c_char) -> bool,
}

#[async_trait]
impl DataStoreTrait for DataStoreC {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        let ref_id = self.data_store_ref.lock().unwrap().unwrap_or(0);
        (self.initialize_fn)(ref_id);
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        let ref_id = self.data_store_ref.lock().unwrap().unwrap_or(0);
        (self.shutdown_fn)(ref_id);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        let ref_id = self.data_store_ref.lock().unwrap().unwrap_or(0);

        let c_key = CString::new(key)
            .map_err(|_| StatsigErr::DataStoreFailure("Key contained a null byte".to_string()))?;

        let result_go = (self.get_fn)(ref_id, c_key.as_ptr());

        if result_go.is_null() {
            return Err(StatsigErr::DataStoreFailure(
                "Failed to get data from data store".to_string(),
            ));
        }

        let result_str = unsafe {
            match CStr::from_ptr(result_go).to_str() {
                Ok(s) => s.to_owned(),
                Err(_) => {
                    return Err(StatsigErr::DataStoreFailure(
                        "Data store returned invalid UTF-8".to_string(),
                    ));
                }
            }
        };

        match serde_json::from_str::<serde_json::Value>(&result_str) {
            Ok(value) => {
                let result_val = match value.get("result") {
                    Some(val) => match val.as_str() {
                        Some(s) => s.to_string(),
                        None => {
                            return Err(StatsigErr::DataStoreFailure(
                                "Failed to get result from data store".to_string(),
                            ));
                        }
                    },
                    None => {
                        return Err(StatsigErr::DataStoreFailure(
                            "Failed to get result from data store".to_string(),
                        ));
                    }
                };

                let time_val = value.get("time").and_then(|t| t.as_u64());

                match serde_json::from_str::<serde_json::Value>(&result_val) {
                    Ok(_val) => Ok(DataStoreResponse {
                        result: Some(result_val),
                        time: time_val,
                    }),
                    Err(e) => Err(StatsigErr::JsonParseError(
                        "Failed to parse data store get response".to_string(),
                        e.to_string(),
                    )),
                }
            }
            Err(e) => Err(StatsigErr::JsonParseError(
                "Failed to parse data store get response".to_string(),
                e.to_string(),
            )),
        }
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        let ref_id = self.data_store_ref.lock().unwrap().unwrap_or(0);

        let data = string_to_c_char(value.to_string());
        let key = string_to_c_char(key.to_string());

        // Convert Option<u64> to *const u64 for the C function
        let time_ptr = match time {
            Some(t) => &t as *const u64,
            None => std::ptr::null(),
        };

        (self.set_fn)(ref_id, key, data, time_ptr);
        Ok(())
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        let ref_id = self.data_store_ref.lock().unwrap().unwrap_or(0);
        (self.support_polling_updates_for_fn)(ref_id, string_to_c_char(path.to_string()))
    }
}

impl DataStoreC {
    /// Set the data store reference ID after creation
    pub fn set_data_store_ref(&self, id: u64) {
        if let Ok(mut guard) = self.data_store_ref.lock() {
            *guard = Some(id);
        }
    }
}

#[no_mangle]
pub extern "C" fn data_store_create(
    initialize_fn: extern "C" fn(data_store_ref: u64),
    shutdown_fn: extern "C" fn(data_store_ref: u64),
    get_fn: extern "C" fn(data_store_ref: u64, key: *const c_char) -> *const c_char,
    set_fn: extern "C" fn(
        data_store_ref: u64,
        key: *const c_char,
        value: *const c_char,
        time: *const u64,
    ),
    support_polling_updates_for_fn: extern "C" fn(data_store_ref: u64, key: *const c_char) -> bool,
) -> u64 {
    InstanceRegistry::register(DataStoreC {
        data_store_ref: Mutex::new(None),
        initialize_fn,
        shutdown_fn,
        get_fn,
        set_fn,
        support_polling_updates_for_fn,
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create DataAdapterStoreC");
        0
    })
}

#[no_mangle]
pub extern "C" fn data_store_set_ref(data_store_ref: u64) {
    if let Some(data_store) = InstanceRegistry::get::<DataStoreC>(&data_store_ref) {
        data_store.set_data_store_ref(data_store_ref);
    }
}

#[no_mangle]
pub extern "C" fn data_store_release(data_store_ref: u64) {
    InstanceRegistry::remove(&data_store_ref);
}
