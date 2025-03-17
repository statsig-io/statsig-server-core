mod utils;

#[cfg(feature = "with_grpc")]
pub mod specs_adapter_tests {
    use crate::utils::mock_data_store::MockDataStore;
    use crate::utils::mock_specs_listener::MockSpecsListener;
    use sigstat_grpc::mock_forward_proxy::{api::ConfigSpecResponse, MockForwardProxy};
    use statsig_rust::data_store_interface::DataStoreResponse;
    use statsig_rust::{
        hashing, SpecAdapterConfig, SpecsAdapter, SpecsAdapterType, SpecsSource,
        StatsigCustomizedSpecsAdapter, StatsigOptions, StatsigRuntime,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_data_store_with_streaming() {
        let rt = StatsigRuntime::get_runtime();
        let (adapter, mock_proxy, data_store, mock_listener) =
            setup_datastore_and_streaming_combo().await;
        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_1".to_string(),
                last_updated: 2,
            }))
            .await;
        data_store
            .mock_response(DataStoreResponse {
                time: Some(1),
                result: Some("init_payload".to_string()),
            })
            .await;

        adapter.initialize(mock_listener.clone());
        adapter.clone().start(&rt).await;
        mock_listener.wait_for_next_update().await;
        // Use data store
        let received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.source,
            SpecsSource::Adapter("DataStore".to_string())
        );
        assert_eq!(received_update.data, "init_payload".to_string()); // examine time
        adapter.clone().schedule_background_sync(&rt).await;

        mock_listener.wait_for_next_update().await;
        let received_update_2 = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update_2.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(received_update_2.data, "bg_sync_1".to_string()); // examine time
    }

    #[tokio::test]
    async fn test_data_store_with_streaming_cache_miss() {
        use statsig_rust::SpecsAdapter;

        let rt = StatsigRuntime::get_runtime();
        let (adapter, mock_proxy, data_store, mock_listener) =
            setup_datastore_and_streaming_combo().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_1".to_string(),
                last_updated: 2,
            }))
            .await;

        adapter.initialize(mock_listener.clone());
        adapter.clone().start(&rt).await;
        mock_listener.wait_for_next_update().await;
        let received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(received_update.data, "bg_sync_1".to_string()); // examine time
        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync_2".to_string(),
                last_updated: 3,
            }))
            .await;
        mock_listener.wait_for_next_update().await;
        let received_update_2 = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update_2.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        assert_eq!(received_update_2.data, "bg_sync_2".to_string()); // examine time
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
            },
            SpecAdapterConfig {
                adapter_type: SpecsAdapterType::NetworkGrpcWebsocket,
                init_timeout_ms: 3000,
                specs_url: Some(format!("http://{}", mock_proxy.proxy_address).to_string()),
            },
        ];
        let mut options = StatsigOptions::new();
        options.data_store = Some(data_store.clone());
        let adapter = Arc::new(StatsigCustomizedSpecsAdapter::new_from_config(
            "secret-key",
            configs,
            &options,
            &hashing::HashUtil::new(),
        ));

        (adapter, mock_proxy, data_store, mock_listener)
    }
}
