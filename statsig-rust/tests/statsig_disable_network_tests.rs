mod utils;

use statsig_rust::{statsig_options::StatsigOptionsBuilder, user::StatsigUserBuilder, Statsig};
use std::sync::Arc;
use utils::mock_scrapi::{self, Endpoint, EndpointStub, MockScrapi};

#[tokio::test]
async fn test_disable_network() {
    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            endpoint: Endpoint::DownloadConfigSpecs,
            response: "".to_string(),
            status: 200,
            method: mock_scrapi::Method::GET,
            delay_ms: 0,
        })
        .await;

    mock_scrapi
        .stub(EndpointStub {
            endpoint: Endpoint::GetIdLists,
            response: "".to_string(),
            status: 200,
            method: mock_scrapi::Method::GET,
            delay_ms: 0,
        })
        .await;

    mock_scrapi
        .stub(EndpointStub {
            endpoint: Endpoint::LogEvent,
            response: "".to_string(),
            status: 200,
            method: mock_scrapi::Method::POST,
            delay_ms: 0,
        })
        .await;

    let options = StatsigOptionsBuilder::new()
        .specs_url(Some(
            mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs),
        ))
        .disable_network(Some(true))
        .id_lists_url(Some(mock_scrapi.url_for_endpoint(Endpoint::GetIdLists)))
        .log_event_url(Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)))
        .enable_id_lists(Some(true))
        .output_log_level(Some(4))
        .build();
    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    let _ = statsig.initialize().await;
    let user = StatsigUserBuilder::new_with_user_id("test_user".to_string()).build();
    statsig.log_event(&user, "test_event", Some("test_value".to_string()), None);
    let _ = statsig.shutdown().await;
    assert_eq!(mock_scrapi.get_requests().len(), 0);
}
