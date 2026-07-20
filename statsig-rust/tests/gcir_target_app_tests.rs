mod utils;

use std::collections::HashSet;
use std::sync::Arc;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::Value;
use statsig_rust::{
    ClientInitResponseOptions, GCIRResponseFormat, HashAlgorithm, Statsig, StatsigOptions,
    StatsigUser,
};

const FIXTURE: &str = "tests/data/dcs_target_app_filtering.json";
const HASHED_KEYS_FIXTURE: &str = "tests/data/dcs_target_app_filtering_hashed_keys.json";
const SUPER_KEY: &str = "client-super-key"; // maps to "app_a" in the fixture
const HASHED_KEY: &str = "client-hashed-key"; // djb2 hash maps to "app_b" in the hashed fixture

struct ExpectedScope {
    gates: &'static [&'static str],
    configs: &'static [&'static str],
    layers: &'static [&'static str],
    param_stores: &'static [&'static str],
}

const ALL: ExpectedScope = ExpectedScope {
    gates: &["gate_app_a", "gate_app_b", "gate_no_target"],
    configs: &[
        "config_app_a",
        "config_app_b",
        "exp_app_a",
        "exp_app_b",
        "cmab_app_a",
        "cmab_app_b",
    ],
    layers: &["layer_app_a", "layer_app_b"],
    param_stores: &["ps_app_a", "ps_app_b"],
};

const APP_A: ExpectedScope = ExpectedScope {
    gates: &["gate_app_a"],
    configs: &["config_app_a", "exp_app_a", "cmab_app_a"],
    layers: &["layer_app_a"],
    param_stores: &["ps_app_a"],
};

const APP_B: ExpectedScope = ExpectedScope {
    gates: &["gate_app_b"],
    configs: &["config_app_b", "exp_app_b", "cmab_app_b"],
    layers: &["layer_app_b"],
    param_stores: &["ps_app_b"],
};

fn gcir_options(
    client_sdk_key: Option<&str>,
    target_app_id: Option<&str>,
) -> ClientInitResponseOptions {
    ClientInitResponseOptions {
        hash_algorithm: Some(HashAlgorithm::None),
        client_sdk_key: client_sdk_key.map(String::from),
        target_app_id: target_app_id.map(String::from),
        ..Default::default()
    }
}

async fn setup_statsig(fixture: &str) -> Statsig {
    let mut statsig_options = StatsigOptions::new();
    statsig_options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data(fixture)));
    statsig_options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(statsig_options)));
    statsig.initialize().await.unwrap();
    statsig
}

async fn get_gcir_from(fixture: &str, options: &ClientInitResponseOptions) -> Value {
    let statsig = setup_statsig(fixture).await;
    let user = StatsigUser::with_user_id("a-user");
    let response = statsig.get_client_init_response_with_options(&user, options);
    serde_json::to_value(&response).unwrap()
}

async fn get_gcir(options: &ClientInitResponseOptions) -> Value {
    get_gcir_from(FIXTURE, options).await
}

fn names_of(response: &Value, section: &str) -> HashSet<String> {
    response[section]
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn to_set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn assert_scoped_to(response: &Value, expected: &ExpectedScope) {
    assert_eq!(names_of(response, "feature_gates"), to_set(expected.gates));
    assert_eq!(
        names_of(response, "dynamic_configs"),
        to_set(expected.configs)
    );
    assert_eq!(names_of(response, "layer_configs"), to_set(expected.layers));
    assert_eq!(
        names_of(response, "param_stores"),
        to_set(expected.param_stores)
    );
}

#[tokio::test]
async fn test_no_key_and_no_target_app_id_returns_everything() {
    // The fixture carries a global "app_id" — this guards against filtering
    // plain server-key GCIRs by the DCS global app id.
    let response = get_gcir(&gcir_options(None, None)).await;
    assert_scoped_to(&response, &ALL);
}

#[tokio::test]
async fn test_client_sdk_key_alone_scopes_to_mapped_app() {
    let response = get_gcir(&gcir_options(Some(SUPER_KEY), None)).await;
    assert_scoped_to(&response, &APP_A);
}

#[tokio::test]
async fn test_target_app_id_overrides_key_derived_app() {
    let response = get_gcir(&gcir_options(Some(SUPER_KEY), Some("app_b"))).await;
    assert_scoped_to(&response, &APP_B);
}

#[tokio::test]
async fn test_target_app_id_without_client_sdk_key_still_filters() {
    let response = get_gcir(&gcir_options(None, Some("app_b"))).await;
    assert_scoped_to(&response, &APP_B);
}

#[tokio::test]
async fn test_empty_target_app_id_treated_as_unset() {
    let response = get_gcir(&gcir_options(None, Some(""))).await;
    assert_scoped_to(&response, &ALL);
}

#[tokio::test]
async fn test_empty_target_app_id_falls_back_to_client_sdk_key() {
    let response = get_gcir(&gcir_options(Some(SUPER_KEY), Some(""))).await;
    assert_scoped_to(&response, &APP_A);
}

#[tokio::test]
async fn test_hashed_sdk_key_scopes_to_mapped_app() {
    let response = get_gcir_from(HASHED_KEYS_FIXTURE, &gcir_options(Some(HASHED_KEY), None)).await;
    assert_eq!(
        names_of(&response, "feature_gates"),
        to_set(&["gate_app_b"])
    );
}

#[tokio::test]
async fn test_target_app_id_applies_to_string_and_v2_formats() {
    let statsig = setup_statsig(FIXTURE).await;
    let user = StatsigUser::with_user_id("a-user");

    let mut options = gcir_options(Some(SUPER_KEY), Some("app_b"));
    let v1: Value = serde_json::from_str(
        &statsig.get_client_init_response_with_options_as_string(&user, &options),
    )
    .unwrap();
    assert_scoped_to(&v1, &APP_B);

    options.response_format = Some(GCIRResponseFormat::InitializeWithSecondaryExposureMapping);
    let v2: Value = serde_json::from_str(
        &statsig.get_client_init_response_with_options_as_string(&user, &options),
    )
    .unwrap();
    assert_eq!(names_of(&v2, "feature_gates"), to_set(APP_B.gates));
    assert_eq!(names_of(&v2, "dynamic_configs"), to_set(APP_B.configs));
    assert_eq!(names_of(&v2, "layer_configs"), to_set(APP_B.layers));

    options.response_format = Some(GCIRResponseFormat::InitializeV2);
    let init_v2: Value = serde_json::from_str(
        &statsig.get_client_init_response_with_options_as_string(&user, &options),
    )
    .unwrap();
    assert_scoped_to(&init_v2, &APP_B);
}
