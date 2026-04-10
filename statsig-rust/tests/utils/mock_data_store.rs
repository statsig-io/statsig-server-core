use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use statsig_rust::{
    data_store_interface::{
        DataStoreBytesResponse, DataStoreResponse, DataStoreTrait, RequestPath,
    },
    StatsigErr,
};

#[derive(Default)]
struct MockDataStoreByteCache {
    proto: Option<Vec<u8>>,
    json: Option<Vec<u8>>,
}

pub struct MockDataStore {
    response: Mutex<Option<DataStoreResponse>>,
    byte_cache: Mutex<MockDataStoreByteCache>,
    get_bytes_error: Mutex<Option<String>>,
    supports_polling: bool,
    byte_cache_enabled: bool,
    get_call_count: Arc<AtomicUsize>,
    get_bytes_call_count: Arc<AtomicUsize>,
    set_call_count: Arc<AtomicUsize>,
    set_bytes_call_count: Arc<AtomicUsize>,
}

impl MockDataStore {
    pub fn new(supports_polling: bool) -> Self {
        Self {
            response: Mutex::new(None),
            byte_cache: Mutex::new(MockDataStoreByteCache::default()),
            get_bytes_error: Mutex::new(None),
            supports_polling,
            byte_cache_enabled: false,
            get_call_count: Arc::new(AtomicUsize::new(0)),
            get_bytes_call_count: Arc::new(AtomicUsize::new(0)),
            set_call_count: Arc::new(AtomicUsize::new(0)),
            set_bytes_call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn new_with_byte_cache(supports_polling: bool) -> Self {
        Self {
            byte_cache_enabled: true,
            ..Self::new(supports_polling)
        }
    }

    pub fn with_proto_cache(proto: &[u8]) -> Self {
        let store = Self::new_with_byte_cache(false);
        store.mock_proto_bytes(proto);
        store
    }

    pub fn with_json_cache(json: &str) -> Self {
        let store = Self::new_with_byte_cache(false);
        store.mock_json_bytes(json);
        store
    }

    pub async fn mock_response(&self, response: DataStoreResponse) {
        let mut lock = self.response.lock().unwrap();
        *lock = Some(response);
    }

    pub fn mock_proto_bytes(&self, proto: &[u8]) {
        self.byte_cache.lock().unwrap().proto = Some(proto.to_vec());
    }

    pub fn mock_json_bytes(&self, json: &str) {
        self.byte_cache.lock().unwrap().json = Some(json.as_bytes().to_vec());
    }

    pub fn mock_get_bytes_error(&self, message: &str) {
        *self.get_bytes_error.lock().unwrap() = Some(message.to_string());
    }

    pub fn stored_proto_bytes(&self) -> Option<Vec<u8>> {
        self.byte_cache.lock().unwrap().proto.clone()
    }

    pub fn stored_json_bytes(&self) -> Option<Vec<u8>> {
        self.byte_cache.lock().unwrap().json.clone()
    }

    pub fn num_get_calls(&self) -> usize {
        self.get_call_count.load(Ordering::SeqCst)
    }

    pub fn num_get_bytes_calls(&self) -> usize {
        self.get_bytes_call_count.load(Ordering::SeqCst)
    }

    pub fn num_set_calls(&self) -> usize {
        self.set_call_count.load(Ordering::SeqCst)
    }

    pub fn num_set_bytes_calls(&self) -> usize {
        self.set_bytes_call_count.load(Ordering::SeqCst)
    }

    fn get_bytes_cache_for_key(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.byte_cache.lock().unwrap();
        if is_proto_cache_key(key) {
            cache.proto.clone()
        } else {
            cache.json.clone()
        }
    }
}

#[async_trait]
impl DataStoreTrait for MockDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        self.get_call_count.fetch_add(1, Ordering::SeqCst);
        let response = self.response.lock().unwrap().take();
        if let Some(response) = response {
            return Ok(response);
        }

        let Some(bytes) = self.get_bytes_cache_for_key(key) else {
            return Err(StatsigErr::DataStoreFailure("Failed to get".to_string()));
        };

        Ok(DataStoreResponse {
            result: Some(String::from_utf8(bytes).map_err(|e| {
                StatsigErr::DataStoreFailure(format!("Cached value is not UTF-8: {e}"))
            })?),
            time: Some(1),
        })
    }

    async fn set(&self, key: &str, value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        self.set_call_count.fetch_add(1, Ordering::SeqCst);
        if self.byte_cache_enabled && !is_proto_cache_key(key) {
            self.byte_cache.lock().unwrap().json = Some(value.as_bytes().to_vec());
        }
        Ok(())
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        self.get_bytes_call_count.fetch_add(1, Ordering::SeqCst);
        if !self.byte_cache_enabled {
            return Err(StatsigErr::BytesNotImplemented);
        }

        if let Some(message) = self.get_bytes_error.lock().unwrap().as_ref() {
            return Err(StatsigErr::DataStoreFailure(message.clone()));
        }

        Ok(DataStoreBytesResponse {
            result: self.get_bytes_cache_for_key(key),
            time: Some(1),
        })
    }

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        _time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        self.set_bytes_call_count.fetch_add(1, Ordering::SeqCst);
        if !self.byte_cache_enabled {
            return Err(StatsigErr::BytesNotImplemented);
        }

        let mut cache = self.byte_cache.lock().unwrap();
        if is_proto_cache_key(key) {
            cache.proto = Some(value.to_vec());
        } else {
            cache.json = Some(value.to_vec());
        }

        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        self.supports_polling
    }
}

fn is_proto_cache_key(key: &str) -> bool {
    key.contains("|statsig-br|")
}
