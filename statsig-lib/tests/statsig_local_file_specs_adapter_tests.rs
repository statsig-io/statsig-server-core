mod utils;

use crate::utils::helpers::load_contents;
use crate::utils::mock_specs_listener::MockSpecsListener;
use lazy_static::lazy_static;
use sigstat::{SpecsAdapter, SpecsSource, StatsigLocalFileSpecsAdapter, StatsigRuntime};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

const SDK_KEY: &str = "server-local-specs-test";
const SPECS_FILE_PATH: &str = "/tmp/3099846163_specs.json"; // djb2(SDK_KEY)_specs.json

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn get_test_lock() -> MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap();

    if PathBuf::from(SPECS_FILE_PATH).exists() {
        fs::remove_file(SPECS_FILE_PATH).expect("Failed to delete the specs file.");
    }

    guard
}

async fn setup() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;
    let dcs = load_contents("eval_proj_dcs.json");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_statsig_local_file_specs_adapter() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, "/tmp", Some(url), false);

    adapter.fetch_and_write_to_file().await.unwrap();

    assert!(
        std::path::Path::new(SPECS_FILE_PATH).exists(),
        "The specs file was not created."
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_concurrent_access() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);

    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let url = url.clone();
            tokio::task::spawn(async move {
                let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, "/tmp", Some(url), false);
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
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = StatsigLocalFileSpecsAdapter::new(SDK_KEY, "/tmp", Some(url), false);
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
async fn test_syncing_from_file() {
    let _lock = get_test_lock();

    let mock_scrapi = setup().await;
    let url = mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs);
    let adapter = Arc::new(StatsigLocalFileSpecsAdapter::new(
        SDK_KEY,
        "/tmp",
        Some(url),
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
