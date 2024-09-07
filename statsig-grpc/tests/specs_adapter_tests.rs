mod mock_forward_proxy;
mod mock_specs_update_listener;
mod test_utils;

#[cfg(test)]
pub mod specs_adapter_tests {
    use crate::mock_forward_proxy::{MockForwardProxy};
    use crate::mock_specs_update_listener::MockListener;
    use crate::test_utils::wait_one_ms;
    use sigstat::{SpecsAdapter, SpecsSource};
    use statsig_grpc::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use sigstat::output_logger::{initialize_simple_output_logger, LogLevel};

    async fn setup() -> (
        Arc<MockForwardProxy>,
        Arc<MockListener>,
        Arc<StatsigGrpcSpecAdapter>,
    ) {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let mock_proxy = MockForwardProxy::spawn().await;
        let adapter = Arc::new(StatsigGrpcSpecAdapter::new(
            "secret-key",
            &format!("http://{}", mock_proxy.proxy_address),
            Some(1),
        ));
        let mock_listener = Arc::new(MockListener::default());
        let handle = Handle::current();
        adapter
            .clone()
            .start(&handle, mock_listener.clone())
            .await
            .unwrap();

        (mock_proxy, mock_listener, adapter)
    }

    #[tokio::test]
    async fn test_manual_syncing() {
        let (
            mock_proxy,
            mock_listener,
            adapter
        ) = setup().await;

        *mock_proxy.stubbed_get_config_spec_response.lock().unwrap() = ConfigSpecResponse {
            spec: "manual-one".to_string(),
            last_updated: 123,
        };

        adapter.manually_sync_specs(None).await.unwrap();

        let received_update_one = mock_listener.force_get_most_recent_update();

        assert_eq!(received_update_one.data, "manual-one");
        assert_eq!(received_update_one.source, SpecsSource::Network);

        *mock_proxy.stubbed_get_config_spec_response.lock().unwrap() = ConfigSpecResponse {
            spec: "manual-two".to_string(),
            last_updated: 123,
        };

        adapter.manually_sync_specs(None).await.unwrap();

        let received_update_two = mock_listener.force_get_most_recent_update();
        assert_eq!(received_update_two.data, "manual-two");
        assert_eq!(received_update_two.source, SpecsSource::Network);
    }

    #[tokio::test]
    async fn test_bg_syncing() {
        let (
            mock_proxy,
            mock_listener,
            _adapter
        ) = setup().await;

        *mock_proxy.stubbed_get_config_spec_response.lock().unwrap() = ConfigSpecResponse {
            spec: "original".to_string(),
            last_updated: 123,
        };

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        mock_listener.wait_for_next_update().await.unwrap();

        let received_update = mock_listener.force_get_most_recent_update();
        assert_eq!(received_update.data, "bg_sync");
        assert_eq!(received_update.source, SpecsSource::Network);
    }

    #[tokio::test]
    async fn test_shutting_down() {
        let (
            mock_proxy,
            mock_listener,
            adapter
        ) = setup().await;

        adapter.shutdown(Duration::from_millis(1)).await.unwrap();

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        wait_one_ms().await;

        let received_update = mock_listener.nullable_get_most_recent_update();
        assert!(received_update.is_none());
    }

    #[tokio::test]
    async fn test_reconnecting() {
        let (
            mock_proxy,
            mock_listener,
            _adapter
        ) = setup().await;

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "first_sync".to_string(),
                last_updated: 123,
            }))
            .await;

        mock_listener.wait_for_next_update().await.unwrap();

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
        assert_eq!(received_update.source, SpecsSource::Network);
    }
}
