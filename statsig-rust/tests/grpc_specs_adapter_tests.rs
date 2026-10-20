mod utils;

#[cfg(all(test, feature = "with_grpc"))]
pub mod specs_adapter_tests {
    use crate::utils::mock_specs_listener::MockSpecsListener;
    use mock_forward_proxy::{api::ConfigSpecResponse, wait_one_ms, MockForwardProxy};
    use sigstat_grpc::*;
    use statsig_rust::output_logger::{initialize_output_logger, LogLevel};
    use statsig_rust::statsig_global::StatsigGlobal;
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
        initialize_output_logger(&Some(LogLevel::Debug), None);

        let mock_proxy = MockForwardProxy::spawn().await;
        let config = SpecAdapterConfig {
            adapter_type: SpecsAdapterType::NetworkGrpcWebsocket,
            specs_url: Some(format!("http://{}", mock_proxy.proxy_address)),
            init_timeout_ms: 3000,
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            domain_name: None,
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
                zstd_dict_id: None,
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
    async fn test_start_after_shutdown_without_running_task_ignores_stale_signal() {
        let statsig_rt = StatsigRuntime::get_runtime();
        let (mock_proxy, mock_listener, adapter) = setup().await;

        assert!(adapter
            .shutdown(Duration::from_millis(1), &statsig_rt)
            .await
            .is_err());

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "started_after_stale_shutdown".to_string(),
                last_updated: 123,
                zstd_dict_id: None,
            }))
            .await;

        adapter.clone().start(&statsig_rt).await.unwrap();

        let mut received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.data.read_to_string().unwrap(),
            "started_after_stale_shutdown"
        );

        adapter
            .shutdown(Duration::from_secs(1), &statsig_rt)
            .await
            .unwrap();
    }

    #[test]
    fn test_runtime_replacement_reschedules_grpc_stream() {
        StatsigGlobal::reset();
        let first_runtime = StatsigRuntime::get_runtime();
        let first_handle = first_runtime.get_handle().unwrap();
        let (mock_proxy, mock_listener, adapter) = first_handle.block_on(setup());

        first_handle.block_on(async {
            mock_proxy
                .send_stream_update(Ok(ConfigSpecResponse {
                    spec: "before_runtime_replacement".to_string(),
                    last_updated: 123,
                    zstd_dict_id: None,
                }))
                .await;
            adapter.clone().start(&first_runtime).await.unwrap();
            let mut initial_update = mock_listener.force_get_most_recent_update();
            assert_eq!(
                initial_update.data.read_to_string().unwrap(),
                "before_runtime_replacement"
            );
        });

        StatsigGlobal::reset();
        let replacement_runtime = StatsigRuntime::get_runtime();
        let replacement_handle = replacement_runtime.get_handle().unwrap();
        replacement_handle.block_on(async {
            adapter
                .clone()
                .schedule_background_sync(&replacement_runtime)
                .await
                .unwrap();
            assert_eq!(
                replacement_runtime
                    .get_running_task_ids()
                    .into_iter()
                    .filter(|(tag, _)| tag == "grpc_streaming")
                    .count(),
                1,
                "replacement runtime must own exactly one new gRPC stream"
            );
        });
        replacement_runtime.shutdown();
        StatsigGlobal::reset();
    }

    #[tokio::test]
    async fn test_can_receive_specs_update() {
        let statsig_rt = StatsigRuntime::get_runtime();
        let (mock_proxy, mock_listener, adapter) = setup().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "first_sync".to_string(),
                last_updated: 123,
                zstd_dict_id: None,
            }))
            .await;

        adapter.clone().start(&statsig_rt).await.unwrap();

        let _ = mock_listener.wait_for_next_update().await;

        mock_proxy.clone().restart().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "reconnected_sync".to_string(),
                last_updated: 123,
                zstd_dict_id: None,
            }))
            .await;

        mock_listener.wait_for_next_update().await.unwrap();

        let mut received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(
            received_update.data.read_to_string().unwrap(),
            "reconnected_sync".to_string()
        );
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
