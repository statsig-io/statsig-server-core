mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use more_asserts::{assert_gt, assert_lt};
use serde_json::json;
use statsig_rust::{FeatureGateEvaluationOptions, Statsig, StatsigOptions, StatsigUser};
use std::sync::{atomic::Ordering, Arc};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;

const DCS_EVAL_PROJ: &str = "eval_proj_dcs";
const DCS_WITH_SAMPLING: &str = "dcs_with_sampling";

async fn setup(dcs_file: &str) -> (Statsig, Arc<MockEventLoggingAdapter>) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let dcs_path = format!("tests/data/{dcs_file}.json");
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data(&dcs_path));

    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(specs_adapter);
    options.event_logging_adapter = Some(logging_adapter.clone());
    options.disable_user_agent_parsing = Some(true);

    let uuid = uuid::Uuid::new_v4();
    let statsig = Statsig::new(&format!("secret-{uuid}"), Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (statsig, logging_adapter)
}

#[tokio::test]
async fn test_gate_exposures() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
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
}

#[tokio::test]
async fn test_dynamic_config_exposures() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    let _ = statsig.get_dynamic_config(&user, "test_email_config");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::config_exposure");

    let metadata: &serde_json::Map<String, serde_json::Value> =
        event["metadata"].as_object().unwrap();
    assert_eq!(metadata["config"], "test_email_config");
    assert_eq!(metadata["configVersion"], "1");
    assert_eq!(metadata["rulePassed"], "false");
}

#[tokio::test]
async fn test_experiment_exposure() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    let _ = statsig.get_experiment(&user, "experiment_with_many_params");
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::config_exposure");
}

#[tokio::test]
async fn test_layer_exposure() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    let layer = statsig.get_layer(&user, "test_layer_with_holdout");
    let _ = layer.get_f64("shared_number_param", 0.0);
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::layer_exposure");

    let metadata: &serde_json::Map<String, serde_json::Value> =
        event["metadata"].as_object().unwrap();
    assert_eq!(metadata["config"], "test_layer_with_holdout");
    assert_eq!(metadata["configVersion"], "4");

    let sec_expo = event["secondaryExposures"].as_array().unwrap();
    assert_eq!(sec_expo.len(), 1);
    assert_eq!(sec_expo[0]["gate"], "layer_holdout");
}

#[tokio::test]
async fn test_custom_event() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    statsig.log_event(&user, "test_event", None, None);
    statsig.log_event(&user, "test_event", None, None); // verify we don't dedupe
    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        2
    );

    let event = logging_adapter.force_get_event_at(0);
    assert_eq!(event["eventName"], "test_event");

    let event = logging_adapter.force_get_event_at(1);
    assert_eq!(event["eventName"], "test_event");
}

#[tokio::test]
async fn test_non_exposed_checks() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    let _ = statsig.check_gate_with_options(
        &user,
        "test_public",
        FeatureGateEvaluationOptions {
            disable_exposure_logging: true,
        },
    );

    statsig.shutdown().await.unwrap();

    assert_eq!(
        logging_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        1
    );

    let event = logging_adapter.force_get_first_event();
    assert_eq!(event["eventName"], "statsig::non_exposed_checks");
}

#[tokio::test]
async fn test_exposure_dedupe() {
    let (statsig, logging_adapter) = setup(DCS_EVAL_PROJ).await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    for _ in 0..10 {
        let _ = statsig.check_gate(&user, "test_public");
    }

    statsig.shutdown().await.unwrap();

    let event_count = logging_adapter
        .no_diagnostics_logged_event_count
        .load(Ordering::SeqCst);
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn test_rule_sampling() {
    let (statsig, logging_adapter) = setup(DCS_WITH_SAMPLING).await;

    fn get_for_user(statsig: &Statsig, user_id: &str) {
        let user = StatsigUser::with_user_id(user_id);
        let _ = statsig.check_gate(&user, "test_rule_sampling");
    }

    for i in 0..2010 {
        get_for_user(&statsig, &format!("user_{i}"));
    }

    statsig.shutdown().await.unwrap();

    // first expo exists and contains no sampling metadata
    let first_event = logging_adapter.force_get_event_at(0);
    let first_event_metadata = &first_event["statsigMetadata"];
    let first_event_user = &first_event["user"];
    assert_eq!(first_event["eventName"], "statsig::gate_exposure");
    assert_eq!(first_event_metadata.get("samplingMode"), None);
    assert_eq!(first_event_user.get("userID"), Some(&json!("user_0")));

    // second expo contains the expected metadata
    let second_event = logging_adapter.force_get_event_at(1);
    let second_event_metadata = &second_event["statsigMetadata"];
    assert_eq!(second_event["eventName"], "statsig::gate_exposure");
    assert_eq!(
        second_event_metadata.get("samplingMode"),
        Some(&json!("on"))
    );
    assert_eq!(second_event_metadata.get("samplingRate"), Some(&json!(201)));
    assert_eq!(
        second_event_metadata.get("shadowLogged"),
        Some(&json!("logged"))
    );

    let event_count = logging_adapter
        .no_diagnostics_logged_event_count
        .load(Ordering::SeqCst);

    // 2010 exposures. Sampled at 1 in 201. So we expect ~10 events.
    assert_gt!(event_count, 2);
    assert_lt!(event_count, 20);
}
