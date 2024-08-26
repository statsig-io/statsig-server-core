mod mock_forward_proxy;
mod mock_specs_update_listener;
mod test_utils;

#[cfg(test)]
pub mod specs_adapter_tests {
    use crate::mock_forward_proxy::{MockForwardProxy, MockForwardProxyVerification};
    use crate::mock_specs_update_listener::MockListener;
    use crate::test_utils::wait_one_ms;
    use sigstat::{SpecsAdapter, SpecsSource};
    use statsig_grpc::*;
    use std::sync::Arc;
    use tokio::runtime::Handle;

    async fn setup() -> (
        Arc<MockForwardProxyVerification>,
        Arc<MockListener>,
        Arc<StatsigGrpcSpecAdapter>,
    ) {
        let mock_proxy = MockForwardProxy::spawn().await;
        let adapter = Arc::new(StatsigGrpcSpecAdapter::new(
            "secret-key",
            &mock_proxy.proxy_address,
        ));
        let mock_listener = Arc::new(MockListener::default());
        let handle = Handle::current();
        adapter
            .clone()
            .start(&handle, mock_listener.clone())
            .await
            .unwrap();

        wait_one_ms().await;

        (mock_proxy, mock_listener, adapter)
    }

    #[tokio::test]
    async fn test_syncing() {
        let (mock_proxy, mock_listener, adapter) = setup().await;

        *mock_proxy.stubbed_get_config_spec_response.lock().unwrap() = ConfigSpecResponse {
            spec: "manual".to_string(),
            last_updated: 123,
        };

        adapter.manually_sync_specs(None).await.unwrap();

        let received_update = mock_listener
            .received_update
            .lock()
            .unwrap()
            .take()
            .unwrap();
        assert_eq!(received_update.data, "manual");
        assert_eq!(received_update.source, SpecsSource::Network);

        mock_proxy
            .send_stream_update(Ok(ConfigSpecResponse {
                spec: "bg_sync".to_string(),
                last_updated: 123,
            }))
            .await;
        wait_one_ms().await;

        let received_update = mock_listener
            .received_update
            .lock()
            .unwrap()
            .take()
            .unwrap();
        assert_eq!(received_update.data, "bg_sync");
        assert_eq!(received_update.source, SpecsSource::Network);
    }
}
