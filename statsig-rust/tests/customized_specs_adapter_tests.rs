mod utils;

#[cfg(feature = "with_grpc")]
pub mod specs_adapter_tests {
    use crate::utils::mock_data_store::MockDataStore;
    use crate::utils::mock_specs_listener::MockSpecsListener;
    use sigstat_grpc::mock_forward_proxy::{api::ConfigSpecResponse, MockForwardProxy};
    use statsig_rust::data_store_interface::DataStoreResponse;
    use statsig_rust::{
        SpecAdapterConfig, SpecsAdapter, SpecsAdapterType, SpecsSource, Statsig,
        StatsigCustomizedSpecsAdapter, StatsigOptions, StatsigRuntime,
    };
    use std::{fs, path::PathBuf, sync::Arc};

    use crate::utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

    #[tokio::test]
    async fn test_data_store_with_streaming() {
        let rt = StatsigRuntime::get_runtime();
        let (adapter, mock_proxy, data_store, mock_listener) =
            setup_datastore_and_streaming_combo().await;
        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_1".to_string(),
                last_updated: 2,
                zstd_dict_id: None,
            }))
            .await;
        data_store
            .mock_response(DataStoreResponse {
                time: Some(1),
                result: Some("init_payload".to_string()),
            })
            .await;

        adapter.initialize(mock_listener.clone());
        let _ = adapter.clone().start(&rt).await;
        let _ = mock_listener.wait_for_next_update().await;
        // Use data store
        let mut received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        assert_eq!(
            received_update.data.read_to_string().unwrap(),
            "init_payload".to_string()
        ); // examine time
        let _ = adapter.clone().schedule_background_sync(&rt).await;

        let _ = mock_listener.wait_for_next_update().await;
        let mut received_update_2 = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update_2.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(
            received_update_2.data.read_to_string().unwrap(),
            "bg_sync_1".to_string()
        );
        // examine time
    }

    #[tokio::test]
    async fn test_network_config_url() {
        let mock_scrapi = MockScrapi::new().await;

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/data/eval_proj_dcs.json");
        let dcs = fs::read_to_string(path).expect("Unable to read file");

        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::String(dcs),
                ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
            })
            .await;

        let statsig = Statsig::new(
            "SECRET-KEY",
            Some(Arc::new(StatsigOptions {
                // spec_adapters_config: Some(vec![SpecAdapterConfig {
                //     adapter_type: SpecsAdapterType::NetworkHttp,
                //     init_timeout_ms: 3000,
                //     specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
                // }]),
                specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
                ..StatsigOptions::default()
            })),
        );
        let result = statsig.initialize_with_details().await.unwrap();

        assert!(result.source.eq(&SpecsSource::Network))
    }

    #[tokio::test]
    async fn test_data_store_with_streaming_cache_miss() {
        use statsig_rust::SpecsAdapter;

        let rt = StatsigRuntime::get_runtime();
        let (adapter, mock_proxy, _data_store, mock_listener) =
            setup_datastore_and_streaming_combo().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_1".to_string(),
                last_updated: 2,
                zstd_dict_id: None,
            }))
            .await;

        adapter.initialize(mock_listener.clone());
        let _ = adapter.clone().start(&rt).await;
        let _ = mock_listener.wait_for_next_update().await;
        let mut received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(
            received_update.data.read_to_string().unwrap(),
            "bg_sync_1".to_string()
        ); // examine time
        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_2".to_string(),
                last_updated: 3,
                zstd_dict_id: None,
            }))
            .await;
        let _ = mock_listener.wait_for_next_update().await;
        let mut received_update_2 = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update_2.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(
            received_update_2.data.read_to_string().unwrap(),
            "bg_sync_2".to_string()
        ); // examine time
        mock_proxy.stop().await;
    }

    async fn setup_datastore_and_streaming_combo() -> (
        Arc<StatsigCustomizedSpecsAdapter>,
        Arc<MockForwardProxy>,
        Arc<MockDataStore>,
        Arc<MockSpecsListener>,
    ) {
        let mock_proxy = MockForwardProxy::spawn().await;
        let mock_listener = Arc::new(MockSpecsListener::default());
        let data_store = Arc::new(MockDataStore::new(false));
        let configs = vec![
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
                adapter_type: SpecsAdapterType::NetworkGrpcWebsocket,
                init_timeout_ms: 3000,
                specs_url: Some(format!("http://{}", mock_proxy.proxy_address).to_string()),
                authentication_mode: None,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                domain_name: None,
            },
        ];
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        let adapter = Arc::new(StatsigCustomizedSpecsAdapter::new_from_config(
            "secret-key",
            "data-store|secret-key",
            configs,
            &options,
        ));

        (adapter, mock_proxy, data_store, mock_listener)
    }
}

mod http_init_timeout_tests {
    use crate::utils::mock_data_store::MockDataStore;
    use crate::utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};
    use statsig_rust::{
        InitializeDetails, SpecAdapterConfig, SpecsAdapterType, SpecsSource, Statsig,
        StatsigOptions,
    };
    use std::sync::Arc;

    const DCS: &str = include_str!("data/eval_proj_dcs.json");

    fn adapter_config(
        adapter_type: SpecsAdapterType,
        init_timeout_ms: u64,
        specs_url: Option<String>,
    ) -> SpecAdapterConfig {
        SpecAdapterConfig {
            adapter_type,
            init_timeout_ms,
            specs_url,
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            domain_name: None,
        }
    }

    async fn scrapi_with_dcs(status: u16, delay_ms: u64) -> MockScrapi {
        let mock_scrapi = MockScrapi::new().await;
        mock_scrapi
            .stub(EndpointStub {
                method: Method::GET,
                response: StubData::String(DCS.to_string()),
                status,
                delay_ms,
                ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
            })
            .await;
        mock_scrapi
    }

    async fn initialize(
        scrapi: &MockScrapi,
        data_store: Arc<MockDataStore>,
        network_timeout_ms: u64,
        global_timeout_ms: u64,
    ) -> InitializeDetails {
        std::env::set_var("STATSIG_RUNNING_TESTS", "true");
        let options = StatsigOptions {
            data_store: Some(data_store),
            disable_all_logging: Some(true),
            disable_country_lookup: Some(true),
            enable_id_lists: Some(false),
            init_timeout_ms: Some(global_timeout_ms),
            spec_adapters_config: Some(vec![
                adapter_config(
                    SpecsAdapterType::NetworkHttp,
                    network_timeout_ms,
                    Some(scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
                ),
                adapter_config(SpecsAdapterType::DataStore, 15_000, None),
            ]),
            ..StatsigOptions::new()
        };
        let statsig = Statsig::new("secret-http-init-timeout", Some(Arc::new(options)));
        let details = statsig.initialize_with_details().await.unwrap();
        statsig.shutdown().await.unwrap();
        details
    }

    fn data_store_was_read(data_store: &MockDataStore) -> bool {
        data_store.num_get_calls() + data_store.num_get_bytes_calls() > 0
    }

    #[tokio::test]
    async fn explicit_http_init_timeout_falls_back_to_data_store_when_network_stalls() {
        let scrapi = scrapi_with_dcs(200, 3_000).await;
        let data_store = Arc::new(MockDataStore::with_json_cache(DCS));

        let details = initialize(&scrapi, data_store.clone(), 200, 10_000).await;

        assert!(details.init_success);
        assert_eq!(
            details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        // The request gives up at the 200ms adapter timeout, so init finishes
        // before the 3s stall would have completed.
        assert!(
            details.duration_ms < 3_000,
            "init took {}ms",
            details.duration_ms
        );
        assert!(data_store_was_read(&data_store));
        assert!(scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) >= 1);
    }

    #[tokio::test]
    async fn zero_http_adapter_timeout_does_not_fall_back_early() {
        let scrapi = scrapi_with_dcs(200, 300).await;
        let data_store = Arc::new(MockDataStore::with_json_cache(DCS));

        let details = initialize(&scrapi, data_store.clone(), 0, 2_000).await;

        assert!(details.init_success);
        assert_eq!(details.source, SpecsSource::Network);
        assert!(!data_store_was_read(&data_store));
    }

    #[tokio::test]
    async fn zero_http_adapter_timeout_uses_default_instead_of_global() {
        let scrapi = scrapi_with_dcs(200, 5_000).await;
        let data_store = Arc::new(MockDataStore::with_json_cache(DCS));

        let details = initialize(&scrapi, data_store.clone(), 0, 10_000).await;

        assert!(details.init_success);
        assert_eq!(
            details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        // The request gives up at the 3000ms default, not the 10s global timeout,
        // so init finishes before the 5s stall would have completed.
        assert!(
            details.duration_ms < 5_000,
            "init took {}ms",
            details.duration_ms
        );
        assert!(data_store_was_read(&data_store));
    }

    #[tokio::test]
    async fn immediate_http_error_still_falls_back_to_data_store() {
        let scrapi = scrapi_with_dcs(503, 0).await;
        let data_store = Arc::new(MockDataStore::with_json_cache(DCS));

        let details = initialize(&scrapi, data_store.clone(), 10_000, 5_000).await;

        assert!(details.init_success);
        assert_eq!(
            details.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        assert!(data_store_was_read(&data_store));
        assert!(
            details.duration_ms < 5_000,
            "init took {}ms",
            details.duration_ms
        );
    }
}
