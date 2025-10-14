mod utils;

use statsig_rust::{
    networking::{NetworkClient, RequestArgs},
    StatsigOptions,
};
use std::sync::Arc;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

async fn setup() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;

    let mut large_response = "a".to_string();
    for _ in 0..22 {
        let again = large_response.clone();
        large_response.push_str(again.as_str());
    }

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: large_response,
            status: 200,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
}

#[tokio::test]
async fn test_streaming_response_to_temp_file() {
    let mock_scrapi = setup().await;
    let network_client = Arc::new(NetworkClient::new("streaming_key", None, None));
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let data = network_client
        .post(
            RequestArgs {
                url,
                ..RequestArgs::new()
            },
            None,
        )
        .await
        .unwrap()
        .data
        .unwrap();

    let stream = data.get_stream_ref();
    let description = format!("{stream:?}").split_at(35).0.to_string();
    assert_eq!(description, "BufReader { reader: SpooledTempFile");
}

#[tokio::test]
async fn test_disable_disk_access_via_request_args() {
    let mock_scrapi = setup().await;
    let network_client = Arc::new(NetworkClient::new("streaming_key", None, None));
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let data = network_client
        .post(
            RequestArgs {
                url,
                disable_file_streaming: Some(true),
                ..RequestArgs::new()
            },
            None,
        )
        .await
        .unwrap()
        .data
        .unwrap();

    let stream = data.get_stream_ref();
    let description = format!("{stream:?}").split_at(35).0.to_string();
    assert_eq!(description, "Cursor { inner: [97, 97, 97, 97, 97");
}

#[tokio::test]
async fn test_disable_disk_access_via_statsig_options() {
    let mock_scrapi = setup().await;
    let options = StatsigOptions {
        disable_disk_access: Some(true),
        ..StatsigOptions::new()
    };
    let network_client = Arc::new(NetworkClient::new("streaming_key", None, Some(&options)));
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let data = network_client
        .post(
            RequestArgs {
                url,
                ..RequestArgs::new()
            },
            None,
        )
        .await
        .unwrap()
        .data
        .unwrap();

    let stream = data.get_stream_ref();
    let description = format!("{stream:?}").split_at(35).0.to_string();
    assert_eq!(description, "Cursor { inner: [97, 97, 97, 97, 97");
}
