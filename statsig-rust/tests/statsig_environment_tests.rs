mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde_json::json;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser, StatsigUserBuilder};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;

const DCS_EVAL_PROJ: &str = "eval_proj_dcs";

async fn setup_with_environment(
    environment: Option<String>,
) -> (Statsig, Arc<MockEventLoggingAdapter>) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let dcs_path = format!("tests/data/{DCS_EVAL_PROJ}.json");
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data(&dcs_path));

    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(specs_adapter);
    options.event_logging_adapter = Some(logging_adapter.clone());
    options.environment = environment;

    let uuid = uuid::Uuid::new_v4();
    let statsig = Statsig::new(&format!("secret-{uuid}"), Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (statsig, logging_adapter)
}

#[tokio::test]
async fn test_environment_from_statsig_options() {
    let (statsig, logging_adapter) = setup_with_environment(Some("production".to_string())).await;

    let user = StatsigUser::with_user_id("test_user".to_string());
    let _ = statsig.check_gate(&user, "test_public");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::gate_exposure");

    let user_data = event["user"].as_object().expect("user should be an object");
    let statsig_environment = user_data
        .get("statsigEnvironment")
        .expect("statsigEnvironment should exist");

    let env_obj = statsig_environment
        .as_object()
        .expect("statsigEnvironment should be an object");
    assert_eq!(env_obj.get("tier"), Some(&json!("production")));
}

#[tokio::test]
async fn test_environment_from_user_overrides_options() {
    // Test Case 2: Set environment on both StatsigOptions and StatsigUser
    let (statsig, logging_adapter) = setup_with_environment(Some("production".to_string())).await;

    let mut user_env = HashMap::new();
    user_env.insert("tier".to_string(), "development".to_string());

    let user = StatsigUserBuilder::new_with_user_id("test_user".to_string())
        .statsig_environment(Some(user_env))
        .build();

    let _ = statsig.check_gate(&user, "test_public");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::gate_exposure");

    let user_data = event["user"].as_object().expect("user should be an object");
    let statsig_environment = user_data
        .get("statsigEnvironment")
        .expect("statsigEnvironment should exist");

    let env_obj = statsig_environment
        .as_object()
        .expect("statsigEnvironment should be an object");

    assert_eq!(
        env_obj.get("tier"),
        Some(&json!("development")),
        "User's tier value should override options tier value"
    );
}

#[tokio::test]
async fn test_environment_only_from_user() {
    // Test Case 3: Set environment just on StatsigUser
    let (statsig, logging_adapter) = setup_with_environment(None).await;

    let mut user_env = HashMap::new();
    user_env.insert("tier".to_string(), "staging".to_string());

    let user = StatsigUserBuilder::new_with_user_id("test_user".to_string())
        .statsig_environment(Some(user_env))
        .build();

    let _ = statsig.check_gate(&user, "test_public");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::gate_exposure");

    // Verify user's environment values are used
    let user_data = event["user"].as_object().expect("user should be an object");
    let statsig_environment = user_data
        .get("statsigEnvironment")
        .expect("statsigEnvironment should exist when set on user");

    let env_obj = statsig_environment
        .as_object()
        .expect("statsigEnvironment should be an object");

    assert_eq!(
        env_obj.get("tier"),
        Some(&json!("staging")),
        "User's tier value should be present"
    );
}

#[tokio::test]
async fn test_no_environment_set() {
    // Test Case 4: Neither StatsigOptions nor StatsigUser has environment
    let (statsig, logging_adapter) = setup_with_environment(None).await;

    let user = StatsigUser::with_user_id("test_user".to_string());
    let _ = statsig.check_gate(&user, "test_public");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::gate_exposure");

    // Verify statsigEnvironment is not present when no environment is set
    let user_data = event["user"].as_object().expect("user should be an object");
    assert!(
        user_data.get("statsigEnvironment").is_none(),
        "statsigEnvironment should not exist when no environment is set"
    );
}

#[tokio::test]
async fn test_environment_override_behavior() {
    // Test Case 5: StatsigOptions has tier="production", user sets tier="testing"
    let (statsig, logging_adapter) = setup_with_environment(Some("production".to_string())).await;

    let mut user_env = HashMap::new();
    user_env.insert("tier".to_string(), "testing".to_string());

    let user = StatsigUserBuilder::new_with_user_id("test_user".to_string())
        .statsig_environment(Some(user_env))
        .build();

    let _ = statsig.check_gate(&user, "test_public");
    statsig.shutdown().await.unwrap();

    let event = logging_adapter.force_get_first_event();

    let user_data = event["user"].as_object().expect("user should be an object");
    let statsig_environment = user_data
        .get("statsigEnvironment")
        .expect("statsigEnvironment should exist");

    let env_obj = statsig_environment
        .as_object()
        .expect("statsigEnvironment should be an object");

    // User's tier value should override options tier value
    assert_eq!(
        env_obj.get("tier"),
        Some(&json!("testing")),
        "User's tier value should override options tier value"
    );
}
