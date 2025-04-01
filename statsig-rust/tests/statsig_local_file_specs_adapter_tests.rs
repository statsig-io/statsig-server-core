mod utils;

use crate::utils::helpers::load_contents;
use crate::utils::mock_specs_listener::MockSpecsListener;
use statsig_rust::{SpecsAdapter, SpecsSource, StatsigLocalFileSpecsAdapter, StatsigRuntime};
use std::fs;
use std::sync::Arc;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "server-local-specs-test";
const SPECS_FILE_NAME: &str = "3099846163_specs.json"; // djb2(SDK_KEY)_specs.json

async fn setup(test_name: &str) -> (MockScrapi, String) {
    let test_path = format!("/tmp/{}", test_name);

    if std::path::Path::new(&test_path).exists() {
        fs::remove_dir_all(&test_path).unwrap();
    }
    fs::create_dir_all(&test_path).unwrap();

    let mock_scrapi = MockScrapi::new().await;
    let dcs = load_contents("eval_proj_dcs.json");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    (mock_scrapi, test_path)
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_statsig_local_file_specs_adapter() {
    let (mock_scrapi, test_path) = setup("test_statsig_local_file_specs_adapter").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, &test_path, Some(url), false, false);

    adapter.fetch_and_write_to_file().await.unwrap();

    let out_path = format!("{}/{}", test_path, SPECS_FILE_NAME);
    assert!(
        std::path::Path::new(&out_path).exists(),
        "The specs file was not created."
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_concurrent_access() {
    let (mock_scrapi, test_path) = setup("test_concurrent_access").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let url = url.clone();
            let test_path = test_path.clone();
            tokio::task::spawn(async move {
                let adapter =
                    StatsigLocalFileSpecsAdapter::new(SDK_KEY, &test_path, Some(url), false, false);
                adapter.fetch_and_write_to_file().await.unwrap();
                let _ = adapter.resync_from_file();
            })
        })
        .collect();

    let results = futures::future::join_all(tasks).await;
    assert_eq!(results.len(), 10);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sending_since_time() {
    let (mock_scrapi, test_path) = setup("test_sending_since_time").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, &test_path, Some(url), false, false);
    adapter.fetch_and_write_to_file().await.unwrap();

    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    assert_eq!(reqs.len(), 1);
    assert!(!reqs[0].url.to_string().contains("sinceTime="));

    adapter.fetch_and_write_to_file().await.unwrap();

    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    assert_eq!(reqs.len(), 2);
    assert!(reqs[1].url.to_string().contains("sinceTime=1729873603830"));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sending_checksum() {
    let (mock_scrapi, test_path) = setup("test_sending_checksum").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, &test_path, Some(url), false, false);
    adapter.fetch_and_write_to_file().await.unwrap();

    let mock_scrapi = MockScrapi::new().await;
    let dcs = load_contents("dcs_with_checksum.json");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, &test_path, Some(url), false, false);
    adapter.fetch_and_write_to_file().await.unwrap();

    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    assert_eq!(reqs.len(), 1);
    assert!(!reqs[0].url.to_string().contains("checksum="));

    adapter.fetch_and_write_to_file().await.unwrap();

    let reqs = mock_scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    assert_eq!(reqs.len(), 2);
    assert!(reqs[1].url.to_string().contains("checksum=1234567890"));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_syncing_from_file() {
    let (mock_scrapi, test_path) = setup("test_syncing_from_file").await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = Arc::new(StatsigLocalFileSpecsAdapter::new(
        SDK_KEY,
        &test_path,
        Some(url),
        false,
        false,
    ));
    adapter.fetch_and_write_to_file().await.unwrap();

    let statsig_rt = StatsigRuntime::get_runtime();
    let listener = Arc::new(MockSpecsListener::default());
    adapter.initialize(listener.clone());
    adapter.clone().start(&statsig_rt).await.unwrap();

    adapter.resync_from_file().unwrap();

    let update = &listener.nullable_get_most_recent_update().unwrap();
    assert_eq!(update.source, SpecsSource::Adapter("FileBased".to_string()));
}
