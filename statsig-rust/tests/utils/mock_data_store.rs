use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    StatsigErr,
};
use tokio::sync::Mutex;

pub struct MockDataStore {
    response: Mutex<Option<DataStoreResponse>>,
    supports_polling: bool,
    get_call_count: Arc<AtomicUsize>,
    set_call_count: Arc<AtomicUsize>,
}

impl MockDataStore {
    pub fn new(supports_polling: bool) -> Self {
        Self {
            response: Mutex::new(None),
            supports_polling,
            get_call_count: Arc::new(AtomicUsize::new(0)),
            set_call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn mock_response(&self, response: DataStoreResponse) {
        let mut lock = self.response.lock().await;
        *lock = Some(response);
    }

    pub fn num_get_calls(&self) -> usize {
        self.get_call_count.load(Ordering::SeqCst)
    }

    pub fn num_set_calls(&self) -> usize {
        self.set_call_count.load(Ordering::SeqCst)
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

    async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
        self.get_call_count.fetch_add(1, Ordering::SeqCst);
        let response = self.response.lock().await.take();
        match response {
            Some(r) => Ok(r),
            None => Err(StatsigErr::DataStoreFailure("Failed to get".to_string())),
        }
    }

    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        self.set_call_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        self.supports_polling
    }
}
