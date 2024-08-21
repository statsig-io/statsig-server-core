use std::collections::HashMap;
use std::sync::Arc;
use statsig::{Statsig, StatsigOptions, StatsigUser};
use statsig::statsig_user::StatsigUserBuilder;
use crate::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::mock_specs_adapter::MockSpecsAdapter;

mod mock_specs_adapter;
mod mock_event_logging_adapter;
async fn setup(environment: Option<String>) -> Statsig {
    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data("tests/eval_proj_dcs.json")));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));
    options.environment = environment;

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

#[tokio::test]
async fn test_string_comparisons_passes() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), "".into()),
    ]));

    let gate_name = "test_string_comparisons";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}


#[tokio::test]
async fn test_nested_gate_condition_passes() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), "".into()),
    ]));

    let gate_name = "test_nested_gate_condition";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}


#[tokio::test]
async fn test_time_before_passes() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), "".into()),
    ]));

    let gate_name = "test_time_before";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_segment_condition_fails() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), "".into()),
    ]));

    let gate_name = "test_segment_condition";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(!gate.value);

    statsig.shutdown().await.unwrap();
}


#[tokio::test]
async fn test_holdout_default_value_gate_fails() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), "".into()),
    ]));

    let gate_name = "test_holdout_default_value_gate";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}


#[tokio::test]
async fn test_app_version() {
    let statsig = setup(None).await;

    let user =
        StatsigUserBuilder::new_with_user_id("a_user".to_string())
            .app_version(Some("1.3".to_string())).build();

    let gate_name = "test_version"; // less than 1.2.3.4

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(!gate.value);

    statsig.shutdown().await.unwrap();
}
