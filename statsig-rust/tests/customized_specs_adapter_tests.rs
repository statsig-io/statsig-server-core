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
