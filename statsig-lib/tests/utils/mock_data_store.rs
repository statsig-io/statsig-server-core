use async_trait::async_trait;
use sigstat::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    StatsigErr,
};
use tokio::sync::Mutex;

pub struct MockDataStore {
    response: Mutex<Option<DataStoreResponse>>,
    supports_polling: bool,
}

impl MockDataStore {
    pub fn new(supports_polling: bool) -> Self {
        Self {
            response: Mutex::new(None),
            supports_polling,
        }
    }

    pub async fn mock_response(&self, response: DataStoreResponse) {
        let mut lock = self.response.lock().await;
        *lock = Some(response);
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
        println!("calling get data store");
        let response = self.response.lock().await.take();
        match response {
            Some(r) => Ok(r),
            None => Err(StatsigErr::DataStoreFailure("Failed to get".to_string())),
        }
    }

    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        self.supports_polling
    }
}
