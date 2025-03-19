mod utils;

use more_asserts::assert_gt;
use statsig_rust::{output_logger::LogLevel, Statsig, StatsigOptions};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use utils::{
    helpers::assert_eventually,
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
};

const SDK_KEY: &str = "secret-key";

async fn setup(options: StatsigOptions) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/eval_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read file");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let statsig = Statsig::new(
        SDK_KEY,
        Some(Arc::new(StatsigOptions {
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            output_log_level: Some(LogLevel::Debug),
            ..options
        })),
    );

    (mock_scrapi, statsig)
}

#[tokio::test]
async fn test_background_syncing() {
    let (scrapi, statsig) = setup(StatsigOptions {
        specs_sync_interval_ms: Some(1),
        ..StatsigOptions::new()
    })
    .await;

    statsig.initialize().await.unwrap();

    assert_eventually(
        || scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 2,
        Duration::from_secs(1),
    )
    .await;
}

#[tokio::test]
async fn test_request_args() {
    let (scrapi, statsig) = setup(StatsigOptions::new()).await;

    statsig.initialize().await.unwrap();

    let requests = scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    let request = &requests[0];
    assert_eq!(request.method, "GET");
    assert!(request
        .url
        .to_string()
        .contains(format!("/v2/download_config_specs/{SDK_KEY}.json").as_str()));

    let headers = request.headers.clone();
    assert_eq!(headers.get("Accept-Encoding").unwrap(), "gzip");

    let client_time = headers
        .get("statsig-client-time")
        .unwrap()
        .to_str()
        .unwrap();
    assert_gt!(client_time.parse::<i64>().unwrap(), 0);

    assert!(headers.get("statsig-sdk-version").is_some());
    assert!(headers.get("statsig-sdk-type").is_some());
    assert_eq!(headers.get("statsig-api-key").unwrap(), SDK_KEY);

    let session_id = headers.get("statsig-server-session-id").unwrap();
    let session_id_str = session_id.to_str().unwrap();
    let parsed_uuid = uuid::Uuid::parse_str(session_id_str).unwrap();
    assert_eq!(parsed_uuid.get_version().unwrap(), uuid::Version::Random);
}
