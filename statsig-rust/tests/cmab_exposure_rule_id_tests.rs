mod utils;

use std::sync::Arc;

#[cfg(feature = "ffi-support")]
use serde_json::Value;
#[cfg(feature = "ffi-support")]
use statsig_rust::ExperimentEvaluationOptions;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use utils::{
    mock_event_logging_adapter::MockEventLoggingAdapter, mock_specs_adapter::MockSpecsAdapter,
};

const DCS_PATH: &str = "tests/data/dcs_cmab_exposure_rule_id.json";

async fn setup() -> (Statsig, Arc<MockEventLoggingAdapter>) {
    let event_logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data(DCS_PATH)));
    options.event_logging_adapter = Some(event_logging_adapter.clone());

    let statsig = Statsig::new("secret-cmab-rule-id-test", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (statsig, event_logging_adapter)
}

fn get_logged_rule_id(adapter: &MockEventLoggingAdapter) -> String {
    adapter.force_get_first_event()["metadata"]["ruleID"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn typed_cmab_exposure_preserves_explore_suffix() {
    let (statsig, adapter) = setup().await;
    let user = StatsigUser::with_user_id("typed-explore-user".to_string());

    let experiment = statsig.get_experiment(&user, "cmab_always_explore");
    let returned_rule_id = experiment.rule_id.clone();
    statsig.shutdown().await.unwrap();

    assert!(returned_rule_id.ends_with(":explore"));
    assert_eq!(get_logged_rule_id(&adapter), returned_rule_id);
}

#[tokio::test]
#[cfg(feature = "ffi-support")]
async fn raw_ffi_cmab_exposure_preserves_explore_suffix() {
    let (statsig, adapter) = setup().await;
    let user = StatsigUser::with_user_id("raw-explore-user".to_string());

    let raw_json = statsig.use_raw_experiment_with_options(
        &user,
        "cmab_always_explore",
        ExperimentEvaluationOptions::default(),
        |raw| raw.unperformant_to_json_string(),
    );
    let raw: Value = serde_json::from_str(&raw_json).unwrap();
    let returned_rule_id = raw["ruleID"].as_str().unwrap().to_string();
    statsig.shutdown().await.unwrap();

    assert!(returned_rule_id.ends_with(":explore"));
    assert_eq!(get_logged_rule_id(&adapter), returned_rule_id);
}

#[tokio::test]
#[cfg(feature = "ffi-support")]
async fn raw_ffi_cmab_exploit_rule_id_remains_bare() {
    let (statsig, adapter) = setup().await;
    let user = StatsigUser::with_user_id("raw-exploit-user".to_string());

    let raw_json = statsig.use_raw_experiment_with_options(
        &user,
        "cmab_never_explore",
        ExperimentEvaluationOptions::default(),
        |raw| raw.unperformant_to_json_string(),
    );
    let raw: Value = serde_json::from_str(&raw_json).unwrap();
    let returned_rule_id = raw["ruleID"].as_str().unwrap().to_string();
    statsig.shutdown().await.unwrap();

    assert_eq!(returned_rule_id, "treatment-group-id");
    assert_eq!(get_logged_rule_id(&adapter), returned_rule_id);
}
