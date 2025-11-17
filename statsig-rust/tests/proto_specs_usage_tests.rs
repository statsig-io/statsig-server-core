mod utils;
use serde_json::json;
use statsig_rust::{output_logger, Statsig, StatsigOptions, StatsigUser};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

use crate::utils::helpers::load_contents;

const KNOWN_CHECKSUM: &str = "2556334789679907000" /* eval_proj_dcs.json['checksum'] */;
const EVAL_PROJ_GATE_COUNT: usize = 69 /* eval_proj_dcs.json['feature_gates'].filter(g => g.entity === feature_gate).length */;
const EVAL_PROJ_DC_COUNT: usize = 7 /* eval_proj_dcs.json['dynamic_configs'].filter(dc => dc.entity === dynamic_config).length */;
const DEMO_PROJ_GATE_COUNT: usize = 7 /* demo_proj_dcs.json['feature_gates'].filter(g => g.entity === feature_gate).length */;

const EVAL_PROJ_PROTO_BYTES: &[u8] = include_bytes!("../tests/data/eval_proj_dcs.pb.br");
const DEMO_PROJ_PROTO_BYTES: &[u8] = include_bytes!("../tests/data/demo_proj_dcs.pb.br");

const INSTANT_SYNC_INTERVAL_MS: u32 = 1;
const DELAYED_SYNC_INTERVAL_MS: u32 = 100;

macro_rules! assert_gate_count {
    ($statsig:ident, $count:expr) => {
        assert_eq!($statsig.get_feature_gate_list().len(), $count);
    };
}

macro_rules! assert_dynamic_config_count {
    ($statsig:ident, $count:expr) => {
        assert_eq!($statsig.get_dynamic_config_list().len(), $count);
    };
}

#[tokio::test]
async fn test_proto_specs_initialize() {
    let (_, statsig) = setup("secret-proto-specs-usage-init", None).await;

    statsig.initialize().await.unwrap();

    let user = StatsigUser::with_user_id("a_user");
    let gate = statsig.check_gate(&user, "test_public");
    assert!(gate);
}

#[tokio::test]
async fn test_proto_specs_syncing() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(INSTANT_SYNC_INTERVAL_MS);
    let (mock_scrapi, statsig) = setup("secret-proto-specs-usage-syncing", Some(options)).await;

    statsig.initialize().await.unwrap();

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 4);

    let user = StatsigUser::with_user_id("a_user");
    let gate = statsig.check_gate(&user, "test_public");
    assert!(gate);
}

#[tokio::test]
async fn test_proto_specs_syncing_corrupt_response() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(INSTANT_SYNC_INTERVAL_MS);
    let (mock_scrapi, statsig) = setup("secret-proto-specs-usage-corrupt", Some(options)).await;

    statsig.initialize().await.unwrap();
    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 2);

    let eval_proj_byte_len = EVAL_PROJ_PROTO_BYTES.len();
    let sub_slice = &EVAL_PROJ_PROTO_BYTES[..eval_proj_byte_len - 100];
    restub_dcs_with_proto(&mock_scrapi, sub_slice).await;

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 2);

    let user = StatsigUser::with_user_id("a_user");
    let gate = statsig.check_gate(&user, "test_public");
    assert!(gate);
}

#[tokio::test]
async fn test_response_swapping() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(INSTANT_SYNC_INTERVAL_MS);
    let (mock_scrapi, statsig) = setup("secret-proto-specs-usage-swap", Some(options)).await;

    statsig.initialize().await.unwrap();

    reset_and_get_checksums(&statsig);

    mock_scrapi.clear_requests();

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 4);

    let (right, left) = reset_and_get_checksums(&statsig);

    // both current and next should have been overwritten with the correct value at this point
    assert_eq!(left, KNOWN_CHECKSUM);
    assert_eq!(right, KNOWN_CHECKSUM);
}

#[tokio::test]
async fn test_proto_then_json() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(DELAYED_SYNC_INTERVAL_MS);
    let (mock_scrapi, statsig) = setup("secret-proto-then-json", Some(options)).await;

    statsig.initialize().await.unwrap();

    assert_gate_count!(statsig, EVAL_PROJ_GATE_COUNT);
    assert_dynamic_config_count!(statsig, EVAL_PROJ_DC_COUNT);

    let empty_dcs = load_dcs_json_with_time(1999999999999);
    restub_dcs_with_json(&mock_scrapi, empty_dcs).await;

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 4);
    assert_gate_count!(statsig, EVAL_PROJ_GATE_COUNT);
    assert_dynamic_config_count!(statsig, 0);
}

#[tokio::test]
async fn test_json_then_proto() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(INSTANT_SYNC_INTERVAL_MS);

    let (mock_scrapi, statsig) = setup("secret-json-then-proto", Some(options)).await;

    restub_dcs_with_json(&mock_scrapi, load_dcs_json_with_time(0_i64)).await;

    statsig.initialize().await.unwrap();

    assert_gate_count!(statsig, EVAL_PROJ_GATE_COUNT);

    restub_dcs_with_proto(&mock_scrapi, DEMO_PROJ_PROTO_BYTES).await;

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 1);

    assert_gate_count!(statsig, DEMO_PROJ_GATE_COUNT);

    restub_dcs_with_proto(&mock_scrapi, EVAL_PROJ_PROTO_BYTES).await;

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 1);

    assert_gate_count!(statsig, EVAL_PROJ_GATE_COUNT);
}

async fn setup(key: &str, options_override: Option<StatsigOptions>) -> (MockScrapi, Statsig) {
    let mock_scrapi = MockScrapi::new().await;

    restub_dcs_with_proto(&mock_scrapi, EVAL_PROJ_PROTO_BYTES).await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{\"success\": true}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let statsig = Statsig::new(
        key,
        Some(Arc::new(StatsigOptions {
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            experimental_flags: Some(HashSet::from(["enable_proto_spec_support".to_string()])),
            ..options_override.unwrap_or_default()
        })),
    );

    (mock_scrapi, statsig)
}

fn make_statsig_opts(sync_interval_ms: u32) -> StatsigOptions {
    StatsigOptions {
        specs_sync_interval_ms: Some(sync_interval_ms),
        output_log_level: Some(output_logger::LogLevel::Debug),
        ..StatsigOptions::new()
    }
}

async fn restub_dcs_with_json(mock_scrapi: &MockScrapi, data: String) {
    mock_scrapi.reset_all().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(data),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
}

async fn restub_dcs_with_proto(mock_scrapi: &MockScrapi, data: &[u8]) {
    mock_scrapi.reset_all().await;

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::Bytes(data.to_vec()),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/octet-stream".to_string(),
            )])),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
}

fn load_dcs_json_with_time(time: i64) -> String {
    let mut empty_dcs: HashMap<String, serde_json::Value> =
        serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    empty_dcs.insert("time".to_string(), json!(time));
    empty_dcs.insert("checksum".to_string(), json!("test-checksum"));
    empty_dcs.insert("dynamic_configs".to_string(), json!({}));
    serde_json::to_string(&empty_dcs).unwrap()
}

fn reset_and_get_checksums(statsig: &Statsig) -> (String, String) {
    let ctx = statsig.get_context();
    let mut data = ctx.spec_store.data.write();

    let curr_checksum = data.values.checksum.clone().unwrap_or_default();
    let next_checksum = data.next_values.checksum.clone().unwrap_or_default();

    data.values.checksum = Some("__test_curr_values".to_string());
    data.next_values.checksum = Some("__test_next_values".to_string());

    (curr_checksum, next_checksum)
}
