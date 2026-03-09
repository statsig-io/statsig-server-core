mod utils;

pub mod data_store_bytes_adapter_tests {
    use crate::utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};
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

    const EVAL_PROJ_PROTO_BYTES: &[u8] = include_bytes!("data/eval_proj_dcs.pb.br");
    const EVAL_PROJ_JSON: &str = include_str!("data/eval_proj_dcs.json");

    struct BytesOnlyDataStore {
        bytes: Vec<u8>,
    }

    impl BytesOnlyDataStore {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes }
        }
    }

    struct RecordingBytesCapableDataStore {
        use_statsig_br_key: bool,
        stored_string_key: Mutex<Option<String>>,
        stored_bytes: Mutex<Option<(String, Vec<u8>)>>,
    }

    impl RecordingBytesCapableDataStore {
        fn new(use_statsig_br_key: bool) -> Self {
            Self {
                use_statsig_br_key,
                stored_string_key: Mutex::new(None),
                stored_bytes: Mutex::new(None),
            }
        }

        fn stored_string_key(&self) -> Option<String> {
            self.stored_string_key.lock().unwrap().clone()
        }

        fn stored_bytes(&self) -> Option<(String, Vec<u8>)> {
            self.stored_bytes.lock().unwrap().clone()
        }
    }

    struct RecordingStringOnlyDataStore {
        use_statsig_br_key: bool,
        stored_string_key: Mutex<Option<String>>,
        bytes_write_attempted: Mutex<bool>,
    }

    impl RecordingStringOnlyDataStore {
        fn new(use_statsig_br_key: bool) -> Self {
            Self {
                use_statsig_br_key,
                stored_string_key: Mutex::new(None),
                bytes_write_attempted: Mutex::new(false),
            }
        }

        fn stored_string_key(&self) -> Option<String> {
            self.stored_string_key.lock().unwrap().clone()
        }

        fn bytes_write_attempted(&self) -> bool {
            *self.bytes_write_attempted.lock().unwrap()
        }
    }

    #[async_trait]
    impl DataStoreTrait for BytesOnlyDataStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            panic!("get() should not be called for bytes-only data store");
        }

        async fn set(
            &self,
            _key: &str,
            _value: &str,
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            panic!("set() should not be called for bytes-only data store");
        }

        async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
            assert!(key.contains("|statsig-br|"));
            Ok(DataStoreBytesResponse {
                result: Some(self.bytes.clone()),
                time: Some(1),
            })
        }

        fn supports_bytes(&self) -> bool {
            true
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            true
        }
    }

    #[async_trait]
    impl DataStoreTrait for RecordingBytesCapableDataStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            panic!("get() should not be called for statsig-br data store");
        }

        async fn set(&self, key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
            *self.stored_string_key.lock().unwrap() = Some(key.to_string());
            Ok(())
        }

        async fn set_bytes(
            &self,
            key: &str,
            value: &[u8],
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            *self.stored_bytes.lock().unwrap() = Some((key.to_string(), value.to_vec()));
            Ok(())
        }

        async fn get_bytes(&self, _key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
            Ok(DataStoreBytesResponse {
                result: None,
                time: None,
            })
        }

        fn supports_bytes(&self) -> bool {
            self.use_statsig_br_key
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn test_data_store_adapter_uses_get_bytes_via_statsig() {
        let data_store = Arc::new(BytesOnlyDataStore::new(EVAL_PROJ_PROTO_BYTES.to_vec()));
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store);

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(
            init_details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
    }

    #[tokio::test]
    async fn test_string_only_data_store_writes_json_as_string() {
        let mock_scrapi = MockScrapi::new().await;
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

        let data_store = Arc::new(RecordingStringOnlyDataStore::new(false));
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs));
        options.log_event_url = Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent));

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);

        sleep(Duration::from_millis(100)).await;
        assert!(data_store
            .stored_string_key()
            .as_deref()
            .is_some_and(|key| key.contains("|plain_text|")));
        assert!(!data_store.bytes_write_attempted());
    }

    #[tokio::test]
    async fn test_non_proto_data_store_writes_json_as_string_even_with_set_bytes() {
        let mock_scrapi = MockScrapi::new().await;
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

        let data_store = Arc::new(RecordingBytesCapableDataStore::new(false));
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs));
        options.log_event_url = Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent));

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);

        sleep(Duration::from_millis(100)).await;

        assert_eq!(data_store.stored_bytes(), None);
        assert!(data_store
            .stored_string_key()
            .as_deref()
            .is_some_and(|key| key.contains("|plain_text|")));
    }

    #[tokio::test]
    async fn test_bytes_capable_statsig_br_data_store_writes_protobuf_bytes() {
        let mock_scrapi = MockScrapi::new().await;
        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::Bytes(EVAL_PROJ_PROTO_BYTES.to_vec()),
                res_headers: Some(HashMap::from([
                    (
                        "Content-Type".to_string(),
                        "application/octet-stream".to_string(),
                    ),
                    ("Content-Encoding".to_string(), "statsig-br".to_string()),
                ])),
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

        let data_store = Arc::new(RecordingBytesCapableDataStore::new(true));
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs));
        options.log_event_url = Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent));

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);

        sleep(Duration::from_millis(100)).await;

        let stored_bytes = data_store.stored_bytes();
        assert!(stored_bytes.is_some());
        let (stored_key, stored_value) = stored_bytes.unwrap();
        assert!(stored_key.contains("|statsig-br|"));
        assert_eq!(stored_value, EVAL_PROJ_PROTO_BYTES);
        assert_eq!(data_store.stored_string_key(), None);
    }

    #[async_trait]
    impl DataStoreTrait for RecordingStringOnlyDataStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            panic!("get() should not be called for recording data store");
        }

        async fn set(&self, key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
            *self.stored_string_key.lock().unwrap() = Some(key.to_string());
            Ok(())
        }

        async fn set_bytes(
            &self,
            _key: &str,
            _value: &[u8],
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            *self.bytes_write_attempted.lock().unwrap() = true;
            Ok(())
        }

        async fn get_bytes(&self, _key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
            Ok(DataStoreBytesResponse {
                result: None,
                time: None,
            })
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }

        fn supports_bytes(&self) -> bool {
            self.use_statsig_br_key
        }
    }

    #[tokio::test]
    async fn test_non_proto_string_only_data_store_skips_protobuf_network_write() {
        let mock_scrapi = MockScrapi::new().await;
        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::Bytes(EVAL_PROJ_PROTO_BYTES.to_vec()),
                res_headers: Some(HashMap::from([
                    (
                        "Content-Type".to_string(),
                        "application/octet-stream".to_string(),
                    ),
                    ("Content-Encoding".to_string(), "statsig-br".to_string()),
                ])),
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

        let data_store = Arc::new(RecordingStringOnlyDataStore::new(false));
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        options.specs_url = Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs));
        options.log_event_url = Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent));

        let statsig = Statsig::new("secret-sdk-key", Some(Arc::new(options)));
        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);

        sleep(Duration::from_millis(100)).await;
        assert_eq!(data_store.stored_string_key(), None);
        assert!(!data_store.bytes_write_attempted());
    }
}
