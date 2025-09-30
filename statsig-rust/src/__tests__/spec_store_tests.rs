use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    networking::ResponseData,
    SpecStore, SpecsSource, SpecsUpdate, StatsigErr, StatsigRuntime,
};

struct TestDataStore {
    get_response: Mutex<Option<DataStoreResponse>>,
    supports_polling: bool,
    calls: Mutex<Vec<(String, Option<String>)>>,
}

#[async_trait]
impl DataStoreTrait for TestDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        self.calls.lock().push(("initialize".to_string(), None));
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        self.calls.lock().push(("shutdown".to_string(), None));
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        self.calls
            .lock()
            .push(("get".to_string(), Some(key.to_string())));

        let mut lock = self.get_response.lock();
        lock.take()
            .ok_or(StatsigErr::DataStoreFailure("Failed to get".to_string()))
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        self.calls
            .lock()
            .push(("set".to_string(), Some(format!("{key}:{value}:{time:?}"))));
        Ok(())
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        self.calls.lock().push((
            "support_polling_updates_for".to_string(),
            Some(path.to_string()),
        ));
        self.supports_polling
    }
}

#[tokio::test]
async fn test_spec_store_data_store_updates_forwarded_to_data_store() {
    let data_store = Arc::new(TestDataStore {
        get_response: Mutex::new(None),
        supports_polling: true,
        calls: Mutex::new(vec![]),
    });

    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        Some(data_store.clone()),
    );

    let contents = include_bytes!("../../tests/data/eval_proj_dcs.json");
    let update_result = spec_store.set_values(SpecsUpdate {
        data: ResponseData::from_bytes(contents.to_vec()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    });

    assert!(update_result.is_ok());

    // data store updates are async, so we need to wait for them to complete
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let calls = data_store.calls.lock();

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "set");

    let call_value = calls[0].1.as_ref().unwrap();
    assert!(call_value.len() > 100);
    assert!(call_value.contains("\"feature_gates\""));
}

#[test]
fn test_failure_to_update() {
    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        None,
    );

    let update_result = spec_store.set_values(SpecsUpdate {
        data: ResponseData::from_bytes(b"test".to_vec()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    });

    assert!(update_result.is_err())
}

#[test]
fn test_no_updates() {
    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        None,
    );

    let update_result = spec_store.set_values(SpecsUpdate {
        data: ResponseData::from_bytes(b"{\"has_updates\": false}".to_vec()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    });

    assert!(update_result.is_ok())
}
