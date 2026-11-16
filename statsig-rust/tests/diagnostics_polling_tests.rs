mod utils;

use serde_json::{json, Value};
use statsig_rust::{Statsig, StatsigOptions};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use utils::diagnostics_helpers::get_diagnostics_events;
use utils::helpers::load_contents;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

const CONFIG_SPECS_KEYS: &[&str] = &["download_config_specs"];
const ID_LIST_KEYS: &[&str] = &["get_id_list_sources", "get_id_list"];

// Both polls run every 10ms and every response takes 20ms, so config and ID
// list syncs are always in flight at the same time.
const SYNC_INTERVAL_MS: u32 = 10;
const RESPONSE_DELAY_MS: u64 = 20;

async fn setup(
    sdk_key: &str,
    sampling_rates: Value,
) -> (MockScrapi, Arc<MockEventLoggingAdapter>, Statsig) {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");
    let mock_scrapi = MockScrapi::new().await;

    let mut dcs: Value = serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    dcs["diagnostics"] = sampling_rates;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(dcs.to_string()),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            delay_ms: RESPONSE_DELAY_MS,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let list_url = format!(
        "{}/{}",
        mock_scrapi.url_for_endpoint(Endpoint::DownloadIdListFile),
        "company_id_list"
    );
    let manifest = json!({
        "company_id_list": {
            "name": "company_id_list",
            "size": 100,
            "url": list_url,
            "creationTime": 1721417546000_u64,
            "fileID": "4t0BEqak3w1UcidsPcpQXN"
        }
    });
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String(manifest.to_string()),
            delay_ms: RESPONSE_DELAY_MS,
            ..EndpointStub::with_endpoint(Endpoint::GetIdLists)
        })
        .await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(load_contents("company_id_list")),
            ..EndpointStub::with_endpoint(Endpoint::DownloadIdListFile)
        })
        .await;

    let logger = Arc::new(MockEventLoggingAdapter::new());
    let options = StatsigOptions {
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        specs_sync_interval_ms: Some(SYNC_INTERVAL_MS),
        enable_id_lists: Some(true),
        id_lists_url: Some(format!(
            "{}/{}",
            mock_scrapi.url_for_endpoint(Endpoint::GetIdLists),
            sdk_key
        )),
        id_lists_sync_interval_ms: Some(SYNC_INTERVAL_MS),
        event_logging_adapter: Some(logger.clone()),
        ..StatsigOptions::new()
    };
    let statsig = Statsig::new(sdk_key, Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (mock_scrapi, logger, statsig)
}

fn config_sync_events(logger: &MockEventLoggingAdapter) -> Vec<Vec<Value>> {
    get_diagnostics_events(logger)
        .into_iter()
        .filter(|(context, _)| context == "config_sync")
        .map(|(_, markers)| markers)
        .collect()
}

fn all_keys_in(markers: &[Value], keys: &[&str]) -> bool {
    markers
        .iter()
        .all(|m| keys.contains(&m["key"].as_str().unwrap_or_default()))
}

/// Flushes until `done` holds for the logged config_sync events, or 5s pass.
async fn flush_until(
    statsig: &Statsig,
    logger: &MockEventLoggingAdapter,
    done: impl Fn(&[Vec<Value>]) -> bool,
) -> Vec<Vec<Value>> {
    let start = Instant::now();
    loop {
        statsig.flush_events().await;
        let events = config_sync_events(logger);
        if done(&events) || start.elapsed() > Duration::from_secs(5) {
            return events;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn test_overlapping_polls_send_separate_config_sync_events() {
    let rates = json!({ "initialize": 10000, "dcs": 10000, "get_id_list": 10000 });
    let (_mock_scrapi, logger, statsig) = setup("secret-diag-overlapping-polls", rates).await;

    let events = flush_until(&statsig, &logger, |events| {
        let config_specs = events
            .iter()
            .filter(|m| all_keys_in(m, CONFIG_SPECS_KEYS))
            .count();
        let id_lists = events
            .iter()
            .filter(|m| all_keys_in(m, ID_LIST_KEYS))
            .count();
        config_specs >= 3 && id_lists >= 3
    })
    .await;
    statsig.shutdown().await.unwrap();

    assert!(!events.is_empty());
    for markers in &events {
        assert!(
            all_keys_in(markers, CONFIG_SPECS_KEYS) || all_keys_in(markers, ID_LIST_KEYS),
            "config_sync event mixes config specs and ID list markers: {markers:?}"
        );
    }
    assert!(
        events
            .iter()
            .filter(|m| all_keys_in(m, CONFIG_SPECS_KEYS))
            .count()
            >= 3
    );
    assert!(
        events
            .iter()
            .filter(|m| all_keys_in(m, ID_LIST_KEYS))
            .count()
            >= 3
    );
}

#[tokio::test]
async fn test_sampled_out_id_list_polls_do_not_leak_into_config_specs_events() {
    let rates = json!({ "initialize": 10000, "dcs": 10000, "get_id_list": 0 });
    let (_mock_scrapi, logger, statsig) = setup("secret-diag-sampled-out-id-lists", rates).await;

    let events = flush_until(&statsig, &logger, |events| events.len() >= 5).await;
    statsig.shutdown().await.unwrap();

    assert!(
        events.len() >= 5,
        "expected config specs events, got {events:?}"
    );
    for markers in &events {
        assert!(
            all_keys_in(markers, CONFIG_SPECS_KEYS),
            "ID list markers were sent although get_id_list is sampled out: {markers:?}"
        );
    }
}
