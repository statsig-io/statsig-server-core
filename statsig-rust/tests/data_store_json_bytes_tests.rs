mod utils;

use async_trait::async_trait;
use statsig_rust::{
    data_store_interface::{
        DataStoreBytesResponse, DataStoreResponse, DataStoreTrait, RequestPath,
    },
    SpecsSource, Statsig, StatsigErr, StatsigOptions,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

const EVAL_PROJ_JSON: &str = include_str!("data/eval_proj_dcs.json");

struct RecordingBytesDataStore {
    values: Mutex<HashMap<String, Vec<u8>>>,
    set_calls: Mutex<u32>,
    set_bytes_calls: Mutex<u32>,
}

impl RecordingBytesDataStore {
    fn new() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            set_calls: Mutex::new(0),
            set_bytes_calls: Mutex::new(0),
        }
    }

    fn last_key_containing(&self, marker: &str) -> Option<String> {
        self.values
            .lock()
            .unwrap()
            .iter()
            .find_map(|(key, _)| key.contains(marker).then_some(key.clone()))
    }

    fn set_calls(&self) -> u32 {
        *self.set_calls.lock().unwrap()
    }

    fn set_bytes_calls(&self) -> u32 {
        *self.set_bytes_calls.lock().unwrap()
    }
}

#[async_trait]
impl DataStoreTrait for RecordingBytesDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
        Ok(DataStoreResponse {
            result: None,
            time: None,
        })
    }

    async fn set(&self, key: &str, value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        *self.set_calls.lock().unwrap() += 1;
        let _ = self
            .values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.as_bytes().to_vec());
        Ok(())
    }

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        _time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        *self.set_bytes_calls.lock().unwrap() += 1;
        let _ = self
            .values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        let result = self.values.lock().unwrap().get(key).cloned();
        Ok(DataStoreBytesResponse { result, time: None })
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        false
    }
}

#[tokio::test]
async fn test_json_bytes_are_written_and_reused_from_datastore_on_reinit() {
    let mock_scrapi = MockScrapi::new().await;
    let data_store = Arc::new(RecordingBytesDataStore::new());
    let specs_url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let log_event_url = mock_scrapi.url_for_endpoint(Endpoint::LogEvent);

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(EVAL_PROJ_JSON.to_string()),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{\"success\": true}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    {
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(specs_url.clone());
        options.log_event_url = Some(log_event_url.clone());

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);
        assert!(mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 0);

        sleep(Duration::from_millis(100)).await;
        assert!(statsig.shutdown().await.is_ok());
    }

    let cached_json_key = data_store
        .last_key_containing("|plain_text|")
        .expect("Expected JSON cache key to be written");
    assert_eq!(
        data_store.get_bytes(&cached_json_key).await.unwrap().result,
        Some(EVAL_PROJ_JSON.as_bytes().to_vec())
    );
    assert_eq!(data_store.set_calls(), 0);
    assert!(data_store.set_bytes_calls() > 0);

    mock_scrapi.reset_all().await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(EVAL_PROJ_JSON.to_string()),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{\"success\": true}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let init_details = {
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(specs_url.clone());
        options.log_event_url = Some(log_event_url.clone());
        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(statsig.shutdown().await.is_ok());
        init_details
    };

    assert!(init_details.init_success);
    assert_eq!(
        init_details.source,
        SpecsSource::Adapter("DataStore".to_string())
    );
    assert_eq!(
        mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs),
        0
    );
    assert_eq!(
        data_store.get_bytes(&cached_json_key).await.unwrap().result,
        Some(EVAL_PROJ_JSON.as_bytes().to_vec())
    );
}
