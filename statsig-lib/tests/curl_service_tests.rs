mod utils;

use crate::utils::mock_scrapi::{Endpoint, EndpointStub, MockScrapi};
use sigstat::networking::{Curl, HttpMethod, RequestArgs};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[tokio::test]
async fn test_multiple_requests() {
    let mock_scrapi = MockScrapi::new().await;

    mock_scrapi
        .stub(EndpointStub {
            delay_ms: 10,
            response: "{\"success\": true}".to_string(),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let results = Arc::new(Mutex::new(vec![]));

    for _ in 0..3 {
        let curl_clone = Curl::get("multiple_requests_key");
        let url_clone = url.clone();
        let results_clone = results.clone();
        tokio::spawn(async move {
            results_clone.lock().await.push(
                curl_clone
                    .send(
                        &HttpMethod::GET,
                        &RequestArgs {
                            url: url_clone,
                            ..RequestArgs::new()
                        },
                    )
                    .await,
            );
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(results.lock().await.len(), 3);
}
