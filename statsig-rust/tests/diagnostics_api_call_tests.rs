mod utils;

use serde_json::{json, Value};
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use utils::diagnostics_helpers::get_markers_for_context;
use utils::helpers::load_contents;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};

const API_CALL_KEYS: &[&str] = &[
    "check_gate",
    "get_dynamic_config",
    "get_experiment",
    "get_layer",
    "get_client_initialize_response",
];

async fn setup(
    sdk_key: &str,
    api_call_rate: f64,
) -> (MockScrapi, Arc<MockEventLoggingAdapter>, Statsig) {
    setup_with_window(sdk_key, api_call_rate, "50").await
}

async fn setup_with_window(
    sdk_key: &str,
    api_call_rate: f64,
    window_ms: &str,
) -> (MockScrapi, Arc<MockEventLoggingAdapter>, Statsig) {
    std::env::set_var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS", "10");
    std::env::set_var("STATSIG_TEST_OVERRIDE_API_CALL_WINDOW_MS", window_ms);

    let mock_scrapi = MockScrapi::new().await;
    let mut dcs: Value = serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    dcs["diagnostics"] = json!({ "initialize": 10000, "api_call": api_call_rate });
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(dcs.to_string()),
            res_headers: Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let logger = Arc::new(MockEventLoggingAdapter::new());
    let options = StatsigOptions {
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        event_logging_adapter: Some(logger.clone()),
        ..StatsigOptions::new()
    };
    let statsig = Statsig::new(sdk_key, Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (mock_scrapi, logger, statsig)
}

fn call_every_method(statsig: &Statsig, user: &StatsigUser) {
    statsig.check_gate(user, "test_public");
    statsig.get_dynamic_config(user, "test_email_config");
    statsig.get_experiment(user, "experiment_with_many_params");
    statsig.get_layer(user, "layer_with_many_params");
    statsig.get_client_init_response(user);
    statsig.get_client_init_response_as_string(user);
}

// The raw and string variants are what most bindings call.
#[cfg(feature = "ffi-support")]
fn call_every_raw_method(statsig: &Statsig, user: &StatsigUser) {
    use statsig_rust::{
        ClientInitResponseOptions, DynamicConfigEvaluationOptions, ExperimentEvaluationOptions,
        FeatureGateEvaluationOptions, LayerEvaluationOptions,
    };

    statsig.use_raw_feature_gate_with_options(
        user,
        "test_public",
        FeatureGateEvaluationOptions::default(),
        |_| (),
    );
    statsig.use_raw_dynamic_config_with_options(
        user,
        "test_email_config",
        DynamicConfigEvaluationOptions::default(),
        |_| (),
    );
    statsig.use_raw_experiment_with_options(
        user,
        "experiment_with_many_params",
        ExperimentEvaluationOptions::default(),
        |_| (),
    );
    statsig.use_raw_layer_with_options(
        user,
        "layer_with_many_params",
        LayerEvaluationOptions::default(),
        |_| (),
    );
    statsig.get_client_init_response_with_options_as_string(
        user,
        &ClientInitResponseOptions::default(),
    );
}

/// Calls every method and flushes until api_call markers for all keys are logged.
async fn collect_api_call_markers(
    statsig: &Statsig,
    logger: &MockEventLoggingAdapter,
    max_wait: Duration,
) -> Vec<Value> {
    collect_api_call_markers_with(statsig, logger, max_wait, call_every_method).await
}

async fn collect_api_call_markers_with(
    statsig: &Statsig,
    logger: &MockEventLoggingAdapter,
    max_wait: Duration,
    call_methods: fn(&Statsig, &StatsigUser),
) -> Vec<Value> {
    let user = StatsigUser::with_user_id("a-user");
    let start = Instant::now();
    loop {
        call_methods(statsig, &user);
        statsig.flush_events().await;

        let markers = get_markers_for_context(logger, "api_call");
        let keys: HashSet<&str> = markers.iter().filter_map(|m| m["key"].as_str()).collect();
        if API_CALL_KEYS.iter().all(|k| keys.contains(k)) || start.elapsed() > max_wait {
            return markers;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn test_sampled_api_calls_are_logged_with_paired_markers() {
    let (_mock_scrapi, logger, statsig) = setup("secret-diag-api-call", 10000.0).await;

    let markers = collect_api_call_markers(&statsig, &logger, Duration::from_secs(5)).await;
    statsig.shutdown().await.unwrap();

    let keys: HashSet<&str> = markers.iter().filter_map(|m| m["key"].as_str()).collect();
    for key in API_CALL_KEYS {
        assert!(
            keys.contains(key),
            "no api_call markers for {key}: {keys:?}"
        );
    }

    let mut starts: HashMap<String, &Value> = HashMap::new();
    let mut ends: HashMap<String, &Value> = HashMap::new();
    for marker in &markers {
        assert!(marker["step"].is_null());
        let id = marker["markerID"].as_str().expect("markerID").to_string();
        match marker["action"].as_str() {
            Some("start") => starts.insert(id, marker),
            Some("end") => ends.insert(id, marker),
            other => panic!("unexpected action {other:?}"),
        };
    }

    // A window can end between a call's start and end, so only check the pairs
    // that landed in the same events.
    let mut checked = 0;
    for (id, start) in &starts {
        let Some(end) = ends.get(id) else {
            continue;
        };
        checked += 1;
        assert_eq!(start["key"], end["key"]);
        assert_eq!(end["success"], true);

        if start["key"] == "get_client_initialize_response" {
            assert!(start["configName"].is_null());
            assert!(end["evaluationDetails"].is_null());
        } else {
            assert!(start["configName"].is_string());
            assert_eq!(start["configName"], end["configName"]);
            assert_eq!(end["evaluationDetails"]["reason"], "Network:Recognized");
        }
    }
    assert!(
        checked >= API_CALL_KEYS.len(),
        "only {checked} complete pairs"
    );
}

#[tokio::test]
async fn test_api_calls_are_not_logged_when_sampled_out() {
    let (_mock_scrapi, logger, statsig) = setup("secret-diag-api-call-off", 0.0).await;

    // several 50ms windows pass, none of them sampled
    let markers = collect_api_call_markers(&statsig, &logger, Duration::from_millis(500)).await;
    statsig.shutdown().await.unwrap();

    assert!(
        markers.is_empty(),
        "unexpected api_call markers: {markers:?}"
    );
}

#[cfg(feature = "ffi-support")]
#[tokio::test]
async fn test_raw_and_string_methods_log_the_same_api_call_keys() {
    let (_mock_scrapi, logger, statsig) = setup("secret-diag-api-call-raw", 10000.0).await;

    let markers = collect_api_call_markers_with(
        &statsig,
        &logger,
        Duration::from_secs(5),
        call_every_raw_method,
    )
    .await;
    statsig.shutdown().await.unwrap();

    let keys: HashSet<&str> = markers.iter().filter_map(|m| m["key"].as_str()).collect();
    for key in API_CALL_KEYS {
        assert!(
            keys.contains(key),
            "no api_call markers for {key}: {keys:?}"
        );
    }
    for marker in markers.iter().filter(|m| m["action"] == "end") {
        assert_eq!(marker["success"], true, "{marker}");
    }
}

#[tokio::test]
async fn test_shutdown_sends_the_open_api_call_window() {
    // a window far longer than the test, so only shutdown can send its markers
    let (_mock_scrapi, logger, statsig) =
        setup_with_window("secret-diag-api-call-shutdown", 10000.0, "600000").await;
    // the first window opens on an early event logger tick, once rates are loaded
    tokio::time::sleep(Duration::from_millis(200)).await;

    let user = StatsigUser::with_user_id("a-user");
    call_every_method(&statsig, &user);
    tokio::time::sleep(Duration::from_millis(100)).await; // markers reach diagnostics asynchronously
    statsig.flush_events().await;
    assert!(get_markers_for_context(&logger, "api_call").is_empty());

    statsig.shutdown().await.unwrap();

    let markers = get_markers_for_context(&logger, "api_call");
    let keys: HashSet<&str> = markers.iter().filter_map(|m| m["key"].as_str()).collect();
    for key in API_CALL_KEYS {
        assert!(
            keys.contains(key),
            "no api_call markers for {key}: {keys:?}"
        );
    }
}
