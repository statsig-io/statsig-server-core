mod utils;

#[cfg(all(test, feature = "with_grpc"))]
pub mod specs_adapter_tests {
    use crate::utils::mock_specs_listener::MockSpecsListener;
    use mock_forward_proxy::{api::ConfigSpecResponse, wait_one_ms, MockForwardProxy};
    use sigstat_grpc::*;
    use statsig_rust::output_logger::{initialize_simple_output_logger, LogLevel};
    use statsig_rust::{
        SpecAdapterConfig, SpecsAdapter, SpecsAdapterType, SpecsSource, StatsigGrpcSpecsAdapter,
        StatsigRuntime,
    };
    use std::sync::Arc;
    use std::time::Duration;

    async fn setup() -> (
        Arc<MockForwardProxy>,
        Arc<MockSpecsListener>,
        Arc<StatsigGrpcSpecsAdapter>,
    ) {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let mock_proxy = MockForwardProxy::spawn().await;
        let config = SpecAdapterConfig {
            adapter_type: SpecsAdapterType::NetworkGrpcWebsocket,
            specs_url: Some(format!("http://{}", mock_proxy.proxy_address)),
            init_timeout_ms: 3000,
        };
        let adapter = Arc::new(StatsigGrpcSpecsAdapter::new("secret-key", &config, None));
        let mock_listener = Arc::new(MockSpecsListener::default());
        adapter.initialize(mock_listener.clone());
        (mock_proxy, mock_listener, adapter)
    }

    #[tokio::test]
    async fn test_shutting_down() {
        let statsig_rt = StatsigRuntime::get_runtime();
        let (mock_proxy, mock_listener, adapter) = setup().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        adapter.clone().start(&statsig_rt).await.unwrap();

        adapter
            .shutdown(Duration::from_millis(1), &statsig_rt)
            .await
            .unwrap();

        wait_one_ms().await;

        let received_update = mock_listener.nullable_get_most_recent_update();
        assert!(received_update.is_some());
    }

    #[tokio::test]
    async fn test_can_receive_specs_update() {
        let statsig_rt = StatsigRuntime::get_runtime();
        let (mock_proxy, mock_listener, adapter) = setup().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "first_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        adapter.clone().start(&statsig_rt).await.unwrap();

        let _ = mock_listener.wait_for_next_update().await;

        mock_proxy.clone().restart().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "reconnected_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        mock_listener.wait_for_next_update().await.unwrap();

        let received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(received_update.data, "reconnected_sync");
        assert_eq!(
            received_update.source,
            SpecsSource::Adapter("GRPC".to_string())
        );
        adapter
            .shutdown(Duration::from_millis(1), &statsig_rt)
            .await
            .unwrap();
    }
}
