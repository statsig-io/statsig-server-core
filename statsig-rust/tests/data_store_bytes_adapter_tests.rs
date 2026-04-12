mod utils;

pub mod data_store_bytes_adapter_tests {
    use crate::assert_eventually;
    use crate::utils::{
        helpers::load_contents,
        mock_data_store::MockDataStore,
        mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData},
    };
    use serde_json::json;
    use statsig_rust::{SpecAdapterConfig, SpecsAdapterType, SpecsSource, Statsig, StatsigOptions};
    use std::{collections::HashMap, sync::Arc};

    const EVAL_PROJ_PROTO_BYTES: &[u8] = include_bytes!("data/eval_proj_dcs.pb.br");
    const BG_SYNC_INTERVAL_MS: u32 = 20;

    #[tokio::test]
    async fn test_empty_data_store_initializes_from_http_then_caches_proto_and_json() {
        let data_store = Arc::new(MockDataStore::new_with_byte_cache(false));
        let (mock_scrapi, statsig) =
            setup_statsig_with_data_store_http("secret-ds-empty-http-fallback", data_store.clone())
                .await;

        stub_dcs_with_proto(&mock_scrapi, EVAL_PROJ_PROTO_BYTES).await;

        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);

        assert_eventually!(
            || data_store.stored_proto_bytes().as_deref() == Some(EVAL_PROJ_PROTO_BYTES)
        );

        let next_json = dcs_json_with_time_and_checksum(1_999_999_999_999, "json-after-proto");
        mock_scrapi.clear_requests();
        stub_dcs_with_json(&mock_scrapi, next_json.clone()).await;

        assert_eventually!(
            || mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 0
        );
        assert_eventually!(
            || data_store.stored_json_bytes().as_deref() == Some(next_json.as_bytes())
        );

        assert!(data_store.num_set_bytes_calls() >= 2);
        assert_eq!(data_store.num_set_calls(), 0);
    }

    #[tokio::test]
    async fn test_proto_data_store_initializes_then_network_json_replaces_cache() {
        let data_store = Arc::new(MockDataStore::with_proto_cache(EVAL_PROJ_PROTO_BYTES));
        let (mock_scrapi, statsig) =
            setup_statsig_with_data_store_http("secret-ds-proto-to-json", data_store.clone()).await;

        let next_json = dcs_json_with_time_and_checksum(1_999_999_999_999, "json-from-network");
        stub_dcs_with_json(&mock_scrapi, next_json.clone()).await;

        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(
            init_details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        assert_eq!(statsig.get_dynamic_config_list().len(), 9);

        assert_eventually!(
            || data_store.stored_json_bytes().as_deref() == Some(next_json.as_bytes())
        );
        assert_eventually!(|| statsig.get_dynamic_config_list().is_empty());
    }

    #[tokio::test]
    async fn test_json_data_store_initializes_then_network_proto_replaces_cache() {
        let cached_json = dcs_json_with_time_and_checksum(0, "cached-json");
        let data_store = Arc::new(MockDataStore::with_json_cache(&cached_json));
        let (mock_scrapi, statsig) =
            setup_statsig_with_data_store_http("secret-ds-json-to-proto", data_store.clone()).await;

        stub_dcs_with_proto(&mock_scrapi, EVAL_PROJ_PROTO_BYTES).await;

        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(
            init_details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        assert!(statsig.get_dynamic_config_list().is_empty());

        assert_eventually!(
            || data_store.stored_proto_bytes().as_deref() == Some(EVAL_PROJ_PROTO_BYTES)
        );
        assert_eventually!(|| statsig.get_dynamic_config_list().len() == 9);
    }

    #[tokio::test]
    async fn test_data_store_bytes_failure_initializes_from_http_without_string_fallback() {
        let cached_json = dcs_json_with_time_and_checksum(0, "cached-json");
        let data_store = Arc::new(MockDataStore::with_json_cache(&cached_json));
        data_store.mock_get_bytes_error("get_bytes failed");
        let (mock_scrapi, statsig) =
            setup_statsig_with_data_store_http("secret-ds-bytes-failure", data_store.clone()).await;

        stub_dcs_with_proto(&mock_scrapi, EVAL_PROJ_PROTO_BYTES).await;

        let init_details = statsig.initialize_with_details().await.unwrap();
        assert!(init_details.init_success);
        assert_eq!(init_details.source, SpecsSource::Network);
        assert_eq!(data_store.num_get_calls(), 0);

        assert_eventually!(
            || data_store.stored_proto_bytes().as_deref() == Some(EVAL_PROJ_PROTO_BYTES)
        );
    }

    async fn setup_statsig_with_data_store_http(
        sdk_key: &str,
        data_store: Arc<MockDataStore>,
    ) -> (MockScrapi, Statsig) {
        std::env::set_var("STATSIG_RUNNING_TESTS", "true");

        let mock_scrapi = MockScrapi::new().await;
        stub_log_event(&mock_scrapi).await;

        let options = StatsigOptions {
            data_store: Some(data_store),
            specs_sync_interval_ms: Some(BG_SYNC_INTERVAL_MS),
            spec_adapters_config: Some(vec![
                SpecAdapterConfig {
                    adapter_type: SpecsAdapterType::DataStore,
                    init_timeout_ms: 3000,
                    specs_url: None,
                    authentication_mode: None,
                    ca_cert_path: None,
                    client_cert_path: None,
                    client_key_path: None,
                    domain_name: None,
                },
                SpecAdapterConfig {
                    adapter_type: SpecsAdapterType::NetworkHttp,
                    init_timeout_ms: 3000,
                    specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
                    authentication_mode: None,
                    ca_cert_path: None,
                    client_cert_path: None,
                    client_key_path: None,
                    domain_name: None,
                },
            ]),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            ..StatsigOptions::new()
        };

        let statsig = Statsig::new(sdk_key, Some(Arc::new(options)));

        (mock_scrapi, statsig)
    }

    async fn stub_dcs_with_proto(mock_scrapi: &MockScrapi, data: &[u8]) {
        mock_scrapi.clear_stubs().await;
        stub_log_event(mock_scrapi).await;

        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::Bytes(data.to_vec()),
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
    }

    async fn stub_dcs_with_json(mock_scrapi: &MockScrapi, data: String) {
        mock_scrapi.clear_stubs().await;
        stub_log_event(mock_scrapi).await;

        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::String(data),
                res_headers: Some(HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )])),
                ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
            })
            .await;
    }

    async fn stub_log_event(mock_scrapi: &MockScrapi) {
        mock_scrapi
            .stub(EndpointStub {
                method: Method::POST,
                response: StubData::String("{\"success\": true}".to_string()),
                ..EndpointStub::with_endpoint(Endpoint::LogEvent)
            })
            .await;
    }

    fn dcs_json_with_time_and_checksum(time: i64, checksum: &str) -> String {
        let mut dcs: HashMap<String, serde_json::Value> =
            serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
        dcs.insert("time".to_string(), json!(time));
        dcs.insert("checksum".to_string(), json!(checksum));
        dcs.insert("dynamic_configs".to_string(), json!({}));
        serde_json::to_string(&dcs).unwrap()
    }
}
