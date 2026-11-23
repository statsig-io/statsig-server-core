mod utils;

use serde_json::Value;
use statsig_rust::{SpecAdapterConfig, SpecsAdapterType, Statsig, StatsigOptions};
use std::collections::HashMap;
use std::sync::Arc;
use utils::diagnostics_helpers::{flush_until_context_logged, summarize_markers};
use utils::helpers::load_contents;
use utils::mock_data_store::MockDataStore;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

const DCS_NETWORK_START: &str = "download_config_specs:network_request:start";
const DCS_NETWORK_END: &str = "download_config_specs:network_request:end";
const DCS_DATA_STORE_READ_START: &str = "download_config_specs:data_store_read:start";
const DCS_DATA_STORE_READ_END: &str = "download_config_specs:data_store_read:end";
const DCS_PROCESS_START: &str = "download_config_specs:process:start";
const DCS_PROCESS_END: &str = "download_config_specs:process:end";

async fn setup_mock_scrapi() -> MockScrapi {
    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(load_contents("eval_proj_dcs.json")),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
    mock_scrapi
}

fn adapter_config(adapter_type: SpecsAdapterType, specs_url: Option<String>) -> SpecAdapterConfig {
    SpecAdapterConfig {
        adapter_type,
        init_timeout_ms: 3000,
        specs_url,
        authentication_mode: None,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        domain_name: None,
    }
}

fn data_store_options(
    mock_scrapi: &MockScrapi,
    data_store: Arc<MockDataStore>,
    logger: Arc<MockEventLoggingAdapter>,
) -> StatsigOptions {
    StatsigOptions {
        data_store: Some(data_store),
        spec_adapters_config: Some(vec![
            adapter_config(SpecsAdapterType::DataStore, None),
            adapter_config(
                SpecsAdapterType::NetworkHttp,
                Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            ),
        ]),
        event_logging_adapter: Some(logger),
        ..StatsigOptions::new()
    }
}

fn markers_with_source<'a>(markers: &'a [Value], source: &str) -> Vec<&'a Value> {
    markers.iter().filter(|m| m["source"] == source).collect()
}

#[tokio::test]
async fn test_http_initialize_markers() {
    let mock_scrapi = setup_mock_scrapi().await;
    let logger = Arc::new(MockEventLoggingAdapter::new());
    let options = StatsigOptions {
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        event_logging_adapter: Some(logger.clone()),
        ..StatsigOptions::new()
    };
    let statsig = Statsig::new("secret-diag-init-http", Some(Arc::new(options)));

    statsig.initialize().await.unwrap();
    let markers = flush_until_context_logged(&statsig, &logger, "initialize").await;

    assert_eq!(
        summarize_markers(&markers),
        vec![
            "overall:start",
            DCS_NETWORK_START,
            DCS_NETWORK_END,
            DCS_PROCESS_START,
            DCS_PROCESS_END,
            "overall:end",
        ]
    );
    assert_eq!(markers.last().unwrap()["source"], "Network");

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_data_store_initialize_markers() {
    let mock_scrapi = setup_mock_scrapi().await;
    let logger = Arc::new(MockEventLoggingAdapter::new());
    let data_store = Arc::new(MockDataStore::with_json_cache(&load_contents(
        "eval_proj_dcs.json",
    )));
    let options = data_store_options(&mock_scrapi, data_store, logger.clone());
    let statsig = Statsig::new("secret-diag-init-data-store", Some(Arc::new(options)));

    statsig.initialize().await.unwrap();
    let markers = flush_until_context_logged(&statsig, &logger, "initialize").await;

    assert_eq!(
        summarize_markers(&markers),
        vec![
            "overall:start",
            DCS_DATA_STORE_READ_START,
            DCS_DATA_STORE_READ_END,
            DCS_PROCESS_START,
            DCS_PROCESS_END,
            "overall:end",
        ]
    );
    let data_store_markers = markers_with_source(&markers, "Adapter(DataStore)");
    assert_eq!(data_store_markers.len(), 5); // four step markers plus overall:end
    assert_eq!(data_store_markers[1]["success"], true);
    assert_eq!(data_store_markers[3]["success"], true);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_empty_data_store_initialize_markers_include_http_fallback() {
    let mock_scrapi = setup_mock_scrapi().await;
    let logger = Arc::new(MockEventLoggingAdapter::new());
    let data_store = Arc::new(MockDataStore::new(false));
    let options = data_store_options(&mock_scrapi, data_store, logger.clone());
    let statsig = Statsig::new("secret-diag-init-data-store-empty", Some(Arc::new(options)));

    statsig.initialize().await.unwrap();
    let markers = flush_until_context_logged(&statsig, &logger, "initialize").await;

    assert_eq!(
        summarize_markers(&markers),
        vec![
            "overall:start",
            // data store read comes back empty
            DCS_DATA_STORE_READ_START,
            DCS_DATA_STORE_READ_END,
            // HTTP fallback
            DCS_NETWORK_START,
            DCS_NETWORK_END,
            DCS_PROCESS_START,
            DCS_PROCESS_END,
            "overall:end",
        ]
    );
    let data_store_markers = markers_with_source(&markers, "Adapter(DataStore)");
    assert_eq!(data_store_markers.len(), 2);
    assert_eq!(data_store_markers[1]["success"], false);
    assert_eq!(markers.last().unwrap()["source"], "Network");

    statsig.shutdown().await.unwrap();
}
