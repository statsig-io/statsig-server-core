mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::statsig_user::StatsigUserBuilder;
use statsig_rust::{dyn_value, Statsig, StatsigOptions, StatsigUser};
use std::collections::HashMap;
use std::sync::Arc;

async fn setup(options_overrides: Option<StatsigOptions>) -> Statsig {
    let options = StatsigOptions {
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/eval_proj_dcs.json",
        ))),
        event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
        ..options_overrides.unwrap_or_default()
    };

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

#[tokio::test]
async fn test_string_comparisons_passes() {
    let statsig = setup(None).await;

    let user = StatsigUser::with_custom_ids(HashMap::from([
        ("companyID".into(), "123".into()),
        ("stableID".into(), String::new()),
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
        ("stableID".into(), String::new()),
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
        ("stableID".into(), String::new()),
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
        ("stableID".into(), String::new()),
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
        ("stableID".into(), String::new()),
    ]));

    let gate_name = "test_holdout_default_value_gate";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_app_version() {
    let statsig = setup(None).await;

    let user = StatsigUserBuilder::new_with_user_id("a_user".to_string())
        .app_version(Some("1.3".to_string()))
        .build();

    let gate_name = "test_version"; // less than 1.2.3.4

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(!gate.value);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_custom_number_value_passes() {
    let statsig = setup(None).await;

    let user = StatsigUserBuilder::new_with_user_id("a_user".to_string())
        .custom(Some(HashMap::from([("level".to_string(), dyn_value!(9))])))
        .build();

    let gate_name = "test_any_with_number_value";

    let gate = statsig.get_feature_gate(&user, gate_name);
    assert!(gate.value);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_experiment_gets_layer_assignment() {
    let statsig = setup(None).await;

    let user =
        StatsigUserBuilder::new_with_user_id("user-not-in-layer-holdout".to_string()).build();

    let experiment_name = "targeted_exp_in_layer_with_holdout";

    let experiment = statsig.get_experiment(&user, experiment_name);
    assert_eq!(experiment.rule_id, "layerAssignment");

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_layer_is_experiment_active() {
    let statsig = setup(None).await;

    let user =
        StatsigUserBuilder::new_with_user_id("user-not-in-layer-holdout".to_string()).build();

    let layer_name = "Basic_test_layer";

    let layer = statsig.get_layer(&user, layer_name);
    assert_eq!(layer.rule_id, "54QJztEPRLXK7ZCvXeY9q4");
    assert_eq!(layer.group_name, Some("Control".to_string()));

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_targeted_exp_in_layer_with_holdout() {
    let statsig = setup(None).await;

    let user = StatsigUser {
        app_version: Some("1.3".into()),
        user_agent: Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into(),
        ),
        ip: Some("1.0.0.0".into()),
        locale: Some("en_US".into()),
        ..StatsigUser::with_user_id("9".into())
    };

    let experiment = statsig.get_experiment(&user, "targeted_exp_in_layer_with_holdout");
    assert_eq!(experiment.rule_id, "layerAssignment");

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_exp_5050_targeting() {
    let statsig = setup(None).await;

    let user = StatsigUser {
        app_version: Some("1.3".into()),
        user_agent: Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into(),
        ),
        ip: Some("1.0.0.0".into()),
        locale: Some("en_US".into()),
        ..StatsigUser::with_user_id("9".into())
    };

    let experiment = statsig.get_experiment(&user, "test_exp_5050_targeting");
    assert_eq!(experiment.rule_id, "targetingGate");
    assert_eq!(experiment.group_name, None);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_many_rules_ua_parser() {
    let options = StatsigOptions {
        enable_user_agent_parsing: Some(true),
        // enable_country_lookup: Some(true),
        ..StatsigOptions::default()
    };
    let statsig = setup(Some(options)).await;

    let user = StatsigUser {
        app_version: Some("1.3".into()),
        ip: Some("1.0.0.0".into()),
        locale: Some("en_US".into()),
        user_agent: Some(
            "Mozilla/5.0 (Windows NT 5.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.5414.87 ADG/11.0.4060 Safari/537.36".into(),
        ),
        ..StatsigUser::with_user_id("123".into())
    };

    let gate = statsig.get_feature_gate(&user, "test_many_rules");
    assert!(gate.value);
    assert_eq!(gate.rule_id, "6p3sVhzePA0Ybcf5r9fwDl");

    statsig.shutdown().await.unwrap();
}
