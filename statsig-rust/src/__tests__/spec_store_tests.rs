use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    networking::ResponseData,
    sdk_event_emitter::SdkEventEmitter,
    SpecStore, SpecsSource, SpecsUpdate, StatsigErr, StatsigOptions, StatsigRuntime,
};

struct TestDataStore {
    get_response: Mutex<Option<DataStoreResponse>>,
    supports_polling: bool,
    set_bytes_error: Option<&'static str>,
    calls: Mutex<Vec<(String, Option<String>)>>,
}

impl TestDataStore {
    fn new(supports_polling: bool) -> Self {
        Self {
            get_response: Mutex::new(None),
            supports_polling,
            set_bytes_error: None,
            calls: Mutex::new(vec![]),
        }
    }

    fn new_with_set_bytes_failure(supports_polling: bool) -> Self {
        Self {
            set_bytes_error: Some("set_bytes failed"),
            ..Self::new(supports_polling)
        }
    }
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

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        self.calls.lock().push((
            "set_bytes".to_string(),
            Some(format!("{key}:{}:{time:?}", value.len())),
        ));

        match self.set_bytes_error {
            Some(message) => Err(StatsigErr::DataStoreFailure(message.to_string())),
            None => Err(StatsigErr::BytesNotImplemented),
        }
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
    let data_store = Arc::new(TestDataStore::new(true));

    let options = StatsigOptions {
        data_store: Some(data_store.clone()),
        ..StatsigOptions::default()
    };

    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        Arc::new(SdkEventEmitter::default()),
        Some(&options),
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

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "set_bytes");
    assert_eq!(calls[1].0, "set");

    let bytes_call_value = calls[0].1.as_ref().unwrap();
    assert!(bytes_call_value.starts_with("test:"));

    let call_value = calls[1].1.as_ref().unwrap();
    assert!(call_value.len() > 100);
    assert!(call_value.contains("\"feature_gates\""));
}

#[tokio::test]
async fn test_spec_store_data_store_string_fallback_requires_bytes_not_implemented() {
    let data_store = Arc::new(TestDataStore::new_with_set_bytes_failure(true));

    let options = StatsigOptions {
        data_store: Some(data_store.clone()),
        ..StatsigOptions::default()
    };

    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        Arc::new(SdkEventEmitter::default()),
        Some(&options),
    );

    let contents = include_bytes!("../../tests/data/eval_proj_dcs.json");
    let update_result = spec_store.set_values(SpecsUpdate {
        data: ResponseData::from_bytes(contents.to_vec()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    });

    assert!(update_result.is_ok());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let calls = data_store.calls.lock();

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "set_bytes");
}

#[tokio::test]
async fn test_spec_store_skips_data_store_write_for_delta_responses() {
    let data_store = Arc::new(TestDataStore::new(true));

    let options = StatsigOptions {
        data_store: Some(data_store.clone()),
        ..StatsigOptions::default()
    };

    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        Arc::new(SdkEventEmitter::default()),
        Some(&options),
    );

    let contents = include_bytes!("../../tests/data/eval_proj_dcs.json");
    let update_result = spec_store.set_values(SpecsUpdate {
        data: ResponseData::from_bytes_with_headers(
            contents.to_vec(),
            Some(HashMap::from([(
                "x-deltas-used".to_string(),
                "true".to_string(),
            )])),
        ),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    });

    assert!(update_result.is_ok());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let calls = data_store.calls.lock();

    assert!(calls.is_empty());
}

#[test]
fn test_failure_to_update() {
    let spec_store = SpecStore::new(
        "test",
        "test".to_string(),
        StatsigRuntime::get_runtime(),
        Arc::new(SdkEventEmitter::default()),
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
        Arc::new(SdkEventEmitter::default()),
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
