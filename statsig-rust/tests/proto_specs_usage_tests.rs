mod utils;
use prost::Message;
use serde_json::json;
use statsig_rust::specs_response::statsig_config_specs as pb;
use statsig_rust::{output_logger, Statsig, StatsigOptions, StatsigUser};
use std::{collections::HashMap, io::Write, sync::Arc};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

use crate::utils::{
    helpers::load_contents, mock_data_store::MockDataStore, mock_log_provider::MockLogProvider,
};

const KNOWN_CHECKSUM: &str = "8506699639233708000" /* eval_proj_dcs.json['checksum'] */;
const EVAL_PROJ_GATE_COUNT: usize = 69 /* eval_proj_dcs.json['feature_gates'].filter(g => g.entity === feature_gate).length */;
const EVAL_PROJ_DC_COUNT: usize = 9 /* eval_proj_dcs.json['dynamic_configs'].filter(dc => dc.entity === dynamic_config).length */;
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
async fn test_proto_specs_request_accept_encoding_header() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let options = make_statsig_opts(DELAYED_SYNC_INTERVAL_MS);
    let (mock_scrapi, statsig) = setup("secret-proto-specs-accept-encoding", Some(options)).await;

    statsig.initialize().await.unwrap();

    let requests = mock_scrapi.get_requests_for_endpoint(Endpoint::DownloadConfigSpecs);
    let request = requests
        .first()
        .expect("expected at least one request for download config specs");

    let accept_encoding = request
        .headers
        .get("accept-encoding")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("");

    assert_eq!(accept_encoding, "statsig-br, gzip, deflate, br");
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

    reset_and_get_checksum(&statsig);

    mock_scrapi.clear_requests();

    assert_eventually!(|| mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) > 4);

    let checksum = reset_and_get_checksum(&statsig);

    // current should have been overwritten with the correct value at this point
    assert_eq!(checksum, KNOWN_CHECKSUM);
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

#[tokio::test]
async fn test_proto_with_data_store() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let log_provider = Arc::new(MockLogProvider::new());
    let data_store = Arc::new(MockDataStore::new(true));

    let options = StatsigOptions {
        data_store: Some(data_store.clone()),
        specs_sync_interval_ms: Some(INSTANT_SYNC_INTERVAL_MS),
        fallback_to_statsig_api: Some(true),
        output_log_level: Some(output_logger::LogLevel::Debug),
        output_logger_provider: Some(log_provider.clone()),
        ..Default::default()
    };
    let (_mock_scrapi, statsig) =
        setup("secret-proto-then-json-with-data-store", Some(options)).await;

    statsig.initialize().await.unwrap();

    assert_eventually!(|| data_store.num_get_calls() > 3);
    assert!(data_store.num_set_calls() == 0);

    let error_logs = log_provider.get_error_logs();
    assert!(error_logs.is_empty());
}

#[tokio::test]
async fn test_proto_experiment_group_condition_initialize_and_sync() {
    let options = StatsigOptions {
        disable_all_logging: Some(true),
        ..make_statsig_opts(DELAYED_SYNC_INTERVAL_MS)
    };
    let (mock_scrapi, statsig) = setup("secret-proto-experiment-group", Some(options)).await;
    restub_dcs_with_proto(&mock_scrapi, &make_experiment_group_proto("Control", 1)).await;

    statsig.initialize().await.unwrap();
    let user = StatsigUser::with_user_id("a_user");
    assert_eq!(
        statsig
            .get_experiment(&user, "source_experiment")
            .group_name
            .as_deref(),
        Some("Control")
    );
    assert!(statsig.check_gate(&user, "group_gate"));

    restub_dcs_with_proto(&mock_scrapi, &make_experiment_group_proto("Test", 2)).await;
    assert_eventually!(|| {
        let gate = statsig.get_feature_gate(&user, "group_gate");
        !gate.value && gate.details.lcut == Some(2) && !gate.details.reason.contains("Unsupported")
    });
    assert_eq!(
        statsig
            .get_experiment(&user, "source_experiment")
            .group_name
            .as_deref(),
        Some("Control")
    );
    statsig.shutdown().await.unwrap();
}

fn make_experiment_group_proto(target_group: &str, time: u64) -> Vec<u8> {
    let id_type = Some(pb::IdType {
        id_type: Some(pb::id_type::IdType::KnownIdType(
            pb::KnownIdType::UserId as i32,
        )),
    });
    let top_level = pb::SpecsTopLevel {
        has_updates: true,
        time,
        checksum: time.to_string(),
        rest: serde_json::to_vec(&json!({"experiment_to_layer": {}})).unwrap(),
        ..Default::default()
    };
    let public = pb::Condition {
        condition_type: pb::ConditionType::Public as i32,
        ..Default::default()
    };
    let group_condition = pb::Condition {
        condition_type: pb::ConditionType::ExperimentGroup as i32,
        field: Some("source_experiment".to_string()),
        operator: Some(pb::Operator::Any as i32),
        target_value: Some(pb::AnyValue {
            value: Some(pb::any_value::Value::RawValue(
                serde_json::to_vec(&json!([target_group])).unwrap(),
            )),
        }),
        id_type: id_type.clone(),
        ..Default::default()
    };
    let experiment = pb::Spec {
        enabled: true,
        entity: pb::EntityType::EntityExperiment as i32,
        is_active: Some(true),
        id_type: id_type.clone(),
        rules: vec![pb::Rule {
            id: "control".to_string(),
            name: "control".to_string(),
            pass_percentage: 100,
            conditions: vec!["public".to_string()],
            group_name: Some("Control".to_string()),
            is_experiment_group: Some(true),
            id_type: id_type.clone(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let gate = pb::Spec {
        enabled: true,
        entity: pb::EntityType::EntityFeatureGate as i32,
        id_type: id_type.clone(),
        rules: vec![pb::Rule {
            id: "group_rule".to_string(),
            name: "group_rule".to_string(),
            pass_percentage: 100,
            conditions: vec!["experiment_group".to_string()],
            return_value: Some(pb::ReturnValue {
                value: Some(pb::return_value::Value::BoolValue(true)),
            }),
            id_type,
            ..Default::default()
        }],
        default_value: Some(pb::ReturnValue {
            value: Some(pb::return_value::Value::BoolValue(false)),
        }),
        ..Default::default()
    };
    let mut bytes = Vec::new();
    for (kind, name, data) in [
        (
            pb::SpecsEnvelopeKind::TopLevel,
            "TOP_LEVEL",
            top_level.encode_to_vec(),
        ),
        (
            pb::SpecsEnvelopeKind::Condition,
            "public",
            public.encode_to_vec(),
        ),
        (
            pb::SpecsEnvelopeKind::Condition,
            "experiment_group",
            group_condition.encode_to_vec(),
        ),
        (
            pb::SpecsEnvelopeKind::DynamicConfig,
            "source_experiment",
            experiment.encode_to_vec(),
        ),
        (
            pb::SpecsEnvelopeKind::FeatureGate,
            "group_gate",
            gate.encode_to_vec(),
        ),
        (pb::SpecsEnvelopeKind::Done, "DONE", vec![]),
    ] {
        pb::SpecsEnvelope {
            kind: kind as i32,
            name: name.to_string(),
            checksum: time.to_string(),
            data: Some(data),
        }
        .encode_length_delimited(&mut bytes)
        .unwrap();
    }
    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 4, 22);
        writer.write_all(&bytes).unwrap();
    }
    compressed
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
            res_headers: Some(HashMap::from([
                (
                    "Content-Type".to_string(),
                    "application/octet-stream".to_string(),
                ),
                ("Content-Encoding".to_string(), "statsig-br".to_string()),
            ])),
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

fn reset_and_get_checksum(statsig: &Statsig) -> String {
    let ctx = statsig.get_context();
    let mut data = ctx.spec_store.data.write();

    let curr_checksum = data.values.checksum.clone().unwrap_or_default();

    data.values.checksum = Some("__test_curr_values".to_string());

    curr_checksum
}
