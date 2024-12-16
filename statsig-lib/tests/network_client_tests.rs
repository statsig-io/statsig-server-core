mod utils;

use sigstat::networking::{NetworkClient, RequestArgs};
use std::{sync::Arc, time::Duration};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

async fn setup() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            delay_ms: 10000,
            method: Method::POST,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
}

#[tokio::test]
async fn test_killing_inflight_requests() {
    let mock_scrapi = setup().await;
    let network_client = Arc::new(NetworkClient::new("inflight_key", None));
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let network_client_clone = network_client.clone();
    let spawned_task = tokio::task::spawn(async move {
        network_client_clone
            .post(
                RequestArgs {
                    url,
                    ..RequestArgs::new()
                },
                None,
            )
            .await.unwrap();
    });

    network_client.shutdown();

    let result = tokio::time::timeout(Duration::from_millis(100), spawned_task).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_per_request_timeout() {
    let mock_scrapi = setup().await;
    let network_client = Arc::new(NetworkClient::new("per_req_key", None));
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let network_client_clone = network_client.clone();
    let spawned_task = tokio::task::spawn(async move {
        network_client_clone
            .post(
                RequestArgs {
                    url,
                    timeout_ms: 10,
                    ..RequestArgs::new()
                },
                None,
            )
            .await.unwrap();
    });

    let result = tokio::time::timeout(Duration::from_millis(100), spawned_task).await;
    assert!(result.is_ok());
}
