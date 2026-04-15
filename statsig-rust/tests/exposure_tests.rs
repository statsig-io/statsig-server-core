mod utils;

use crate::utils::helpers::{enforce_array, enforce_object, enforce_string, enforce_u64};
use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use assert_json_diff::assert_json_include;
use chrono::Utc;
use serde_json::{json, Map, Value};
use statsig_rust::{Statsig, StatsigOptions, StatsigUser, StatsigUserBuilder};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const SEC_EXPO_AS_PRIMARY_FLAG: &str = "sec_expo_as_primary:abc123";
const SEC_EXPO_AS_PRIMARY_FLAG_BUCKET: u64 = 307;

#[tokio::test]
async fn test_gate_exposures_initialized() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_public");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let reason = get_reason_from_adapter(&logging_adapter).await;
    assert_eq!(reason, "Bootstrap:Recognized");
}

#[tokio::test]
async fn test_gate_exposures_formatting() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_50_50");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let received = logging_adapter.force_get_received_payloads();

    let statsig_meta = enforce_object(&received.statsig_metadata);
    assert_eq!(statsig_meta["sdkType"], "statsig-server-core");
    assert!(statsig_meta["sdkVersion"].as_str().is_some());

    let exposure = logging_adapter.force_get_first_event();
    assert_eq!(exposure["eventName"], "statsig::gate_exposure");

    let sec_expos = enforce_array(&exposure["secondaryExposures"]);
    let holdout_exposure = enforce_object(&sec_expos[0]);
    assert_eq!(holdout_exposure["gate"], "global_holdout");
    assert_eq!(holdout_exposure["gateValue"], "false");
    assert_eq!(holdout_exposure["ruleID"], "3QoA4ncNdVGBaMt3N1KYjz:0.50:1");
}

#[tokio::test]
async fn test_gate_exposures_uninitialized() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);

    let _ = statsig.check_gate(&user, "test_public");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let reason = get_reason_from_adapter(&logging_adapter).await;
    assert_eq!(reason, "Uninitialized");
}

#[tokio::test]
async fn test_gate_exposures_unrecognized() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "not_a_gate");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let reason = get_reason_from_adapter(&logging_adapter).await;
    assert_eq!(reason, "Bootstrap:Unrecognized");
}

#[tokio::test]
async fn test_gate_exposures_bad_network() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_trowing_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    let _ = statsig.initialize().await;

    let _ = statsig.check_gate(&user, "not_a_gate");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let reason = get_reason_from_adapter(&logging_adapter).await;
    assert_eq!(reason, "NoValues");
}

#[tokio::test]
async fn test_gate_exposures_not_awaited() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_delayed_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = Arc::new(create_statsig(&specs_adapter, &logging_adapter));

    let shared_statsig = statsig.clone();
    tokio::task::spawn(async move {
        shared_statsig.initialize().await.unwrap();
    });

    sleep(Duration::from_millis(1)).await;

    let _ = statsig.check_gate(&user, "not_a_gate");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let reason = get_reason_from_adapter(&logging_adapter).await;
    assert_eq!(reason, "Loading:Unrecognized");
}

#[tokio::test]
async fn test_check_gate_exposure_with_secondary_exposures() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let event = logging_adapter.force_get_first_event();
    let secondary_expo = enforce_array(&event["secondaryExposures"]);

    let one = enforce_object(&secondary_expo[0]);
    assert_eq!(one["gate"], "test_email");
    assert_eq!(one["ruleID"], "default");
    assert_eq!(one["gateValue"], "false");

    let two = enforce_object(&secondary_expo[1]);
    assert_eq!(two["gate"], "test_environment_tier");
    assert_eq!(two["ruleID"], "default");
    assert_eq!(two["gateValue"], "false");
}

#[tokio::test]
async fn test_get_feature_gate_exposure_with_secondary_exposures() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let _ = statsig.get_feature_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let event = logging_adapter.force_get_first_event();
    let secondary_expo = enforce_array(&event["secondaryExposures"]);

    let one = enforce_object(&secondary_expo[0]);
    assert_eq!(one["gate"], "test_email");
    assert_eq!(one["ruleID"], "default");
    assert_eq!(one["gateValue"], "false");

    let two = enforce_object(&secondary_expo[1]);
    assert_eq!(two["gate"], "test_environment_tier");
    assert_eq!(two["ruleID"], "default");
    assert_eq!(two["gateValue"], "false");
}

#[tokio::test]
async fn test_secondary_exposures_logged_as_primary_when_flag_enabled() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(1000);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_experimental_flags(
        "secret-shhh-sec-expo-enabled",
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 3);

    for event in &events {
        assert!(enforce_array(&event["secondaryExposures"]).is_empty());
    }

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [],
        })
    );
    assert_json_include!(
        actual: events[1],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })
    );
    assert_json_include!(
        actual: events[2],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })
    );
}

#[tokio::test]
async fn test_gate_dynamic_config_experiment_and_layer_log_secondary_exposures_as_primary() {
    assert_secondary_exposures_logged_as_primary_for_eval(
        "secret-shhh-sec-expo-gate-kind",
        StatsigUser::with_user_id("a_user_id"),
        |statsig, user| {
            let _ = statsig.check_gate(user, "test_nested_gate_condition");
        },
        json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [],
        }),
        vec![
            json!({
                "eventName": "statsig::gate_exposure",
                "metadata": {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                "secondaryExposures": [],
            }),
            json!({
                "eventName": "statsig::gate_exposure",
                "metadata": {
                    "gate": "test_environment_tier",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                "secondaryExposures": [],
            }),
        ],
    )
    .await;

    assert_secondary_exposures_logged_as_primary_for_eval(
        "secret-shhh-sec-expo-config-kind",
        StatsigUser::with_user_id("a_user_id"),
        |statsig, user| {
            let _ = statsig.get_dynamic_config(user, "operating_system_config");
        },
        json!({
            "eventName": "statsig::config_exposure",
            "metadata": {
                "config": "operating_system_config",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        }),
        vec![json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })],
    )
    .await;

    assert_secondary_exposures_logged_as_primary_for_eval(
        "secret-shhh-sec-expo-experiment-kind",
        StatsigUserBuilder::new_with_user_id("a-user".to_string())
            .email(Some("daniel@statsig.com".to_string()))
            .build(),
        |statsig, user| {
            let _ = statsig.get_experiment(user, "running_exp_in_unlayered_with_holdout");
        },
        json!({
            "eventName": "statsig::config_exposure",
            "metadata": {
                "config": "running_exp_in_unlayered_with_holdout",
                "ruleID": "5suobe8yyvznqasn9Ph1dI",
            },
            "secondaryExposures": [],
        }),
        vec![
            json!({
                "eventName": "statsig::gate_exposure",
                "metadata": {
                    "gate": "global_holdout",
                    "gateValue": "false",
                    "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1",
                },
                "secondaryExposures": [],
            }),
            json!({
                "eventName": "statsig::gate_exposure",
                "metadata": {
                    "gate": "exp_holdout",
                    "gateValue": "false",
                    "ruleID": "1rEqLOpCROaRafv7ubGgax",
                },
                "secondaryExposures": [],
            }),
        ],
    )
    .await;

    assert_secondary_exposures_logged_as_primary_for_eval(
        "secret-shhh-sec-expo-layer-kind",
        StatsigUser::with_user_id("a_user_id"),
        |statsig, user| {
            let layer = statsig.get_layer(user, "layer_in_global_holdout");
            let _ = layer.get_string("shared_param", String::new());
        },
        json!({
            "eventName": "statsig::layer_exposure",
            "metadata": {
                "config": "layer_in_global_holdout",
                "parameterName": "shared_param",
            },
            "secondaryExposures": [],
        }),
        vec![json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "global_holdout",
                "gateValue": "false",
                "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1",
            },
            "secondaryExposures": [],
        })],
    )
    .await;
}

#[tokio::test]
async fn test_secondary_exposures_remain_on_primary_when_sec_expo_number_is_zero() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(0);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_experimental_flags(
        "secret-shhh-sec-expo-threshold-off",
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 1);

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [
                {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                {
                    "gate": "test_environment_tier",
                    "gateValue": "false",
                    "ruleID": "default",
                },
            ],
        })
    );
}

#[tokio::test]
async fn test_secondary_exposures_remain_on_primary_when_sec_expo_number_is_missing() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_experimental_flags(
        "secret-shhh-sec-expo-missing-config",
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 1);

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [
                {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                {
                    "gate": "test_environment_tier",
                    "gateValue": "false",
                    "ruleID": "default",
                },
            ],
        })
    );
}

#[tokio::test]
async fn test_secondary_exposures_roll_out_from_zero_to_thousand() {
    assert_sec_expo_rollout_result(0, "secret-shhh-sec-expo-rollout-0", false).await;
    assert_sec_expo_rollout_result(
        SEC_EXPO_AS_PRIMARY_FLAG_BUCKET,
        "secret-shhh-sec-expo-rollout-equal-bucket",
        false,
    )
    .await;
    assert_sec_expo_rollout_result(
        SEC_EXPO_AS_PRIMARY_FLAG_BUCKET + 1,
        "secret-shhh-sec-expo-rollout-after-bucket",
        true,
    )
    .await;
    assert_sec_expo_rollout_result(1000, "secret-shhh-sec-expo-rollout-1000", true).await;
}

#[tokio::test]
async fn test_secondary_exposures_remain_on_primary_when_flag_not_enabled() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(300);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_sdk_key(
        "secret-shhh-sec-expo-no-flag",
        &specs_adapter,
        &logging_adapter,
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(enforce_array(&events[0]["secondaryExposures"]).len(), 2);

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [
                {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                {
                    "gate": "test_environment_tier",
                    "gateValue": "false",
                    "ruleID": "default",
                },
            ],
        })
    );
}

#[tokio::test]
async fn test_secondary_exposures_logged_as_primary_are_deduped() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(1000);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_experimental_flags(
        "secret-shhh-sec-expo-deduped",
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    let _ = statsig.get_dynamic_config(&user, "operating_system_config");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 4);

    for event in &events {
        assert!(enforce_array(&event["secondaryExposures"]).is_empty());
    }

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [],
        })
    );
    assert_json_include!(
        actual: events[1],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_email",
                "gateValue": "false",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })
    );
    assert_json_include!(
        actual: events[2],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_environment_tier",
                "gateValue": "false",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })
    );
    assert_json_include!(
        actual: events[3],
        expected: json!({
            "eventName": "statsig::config_exposure",
            "metadata": {
                "config": "operating_system_config",
                "ruleID": "default",
            },
            "secondaryExposures": [],
        })
    );
}

#[tokio::test]
async fn test_secondary_exposures_on_primary_are_not_deduped_when_flag_not_enabled() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(300);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_sdk_key(
        "secret-shhh-sec-expo-no-flag-dedupe",
        &specs_adapter,
        &logging_adapter,
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    let _ = statsig.get_dynamic_config(&user, "operating_system_config");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(enforce_array(&events[0]["secondaryExposures"]).len(), 2);
    assert_eq!(enforce_array(&events[1]["secondaryExposures"]).len(), 1);

    assert_json_include!(
        actual: events[0],
        expected: json!({
            "eventName": "statsig::gate_exposure",
            "metadata": {
                "gate": "test_nested_gate_condition",
                "gateValue": "true",
                "ruleID": "6MlXHRavmo1ujM1NkZNjhQ",
            },
            "secondaryExposures": [
                {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
                {
                    "gate": "test_environment_tier",
                    "gateValue": "false",
                    "ruleID": "default",
                },
            ],
        })
    );
    assert_json_include!(
        actual: events[1],
        expected: json!({
            "eventName": "statsig::config_exposure",
            "metadata": {
                "config": "operating_system_config",
                "ruleID": "default",
            },
            "secondaryExposures": [
                {
                    "gate": "test_email",
                    "gateValue": "false",
                    "ruleID": "default",
                },
            ],
        })
    );
}

#[tokio::test]
async fn test_get_layer_copies_undelegated_exposures() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let layer = statsig.get_layer(&user, "layer_in_global_holdout");
    let _ = layer.get_string("shared_param", String::new());

    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let event = logging_adapter.force_get_first_event();
    let secondary_expo = enforce_array(&event["secondaryExposures"]);

    let one = enforce_object(&secondary_expo[0]);
    assert_eq!(one["gate"], "global_holdout");
    assert_eq!(one["ruleID"], "3QoA4ncNdVGBaMt3N1KYjz:0.50:1");
    assert_eq!(one["gateValue"], "false");
}

#[tokio::test]
async fn test_get_layer_with_holdouts() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("user-in-layer-holdout-4");

    let statsig = create_statsig(&specs_adapter, &logging_adapter);
    statsig.initialize().await.unwrap();

    let layer = statsig.get_layer(&user, "test_layer_in_holdout");
    let _ = layer.get_string("layer_val", String::new());

    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let event = logging_adapter.force_get_first_event();
    let secondary_expo = enforce_array(&event["secondaryExposures"]);

    let one = enforce_object(&secondary_expo[0]);
    assert_eq!(one["gate"], "global_holdout");
    assert_eq!(one["ruleID"], "3QoA4ncNdVGBaMt3N1KYjz:0.50:1");
    assert_eq!(one["gateValue"], "false");

    let two = enforce_object(&secondary_expo[1]);
    assert_eq!(two["gate"], "layer_holdout");
    assert_eq!(two["ruleID"], "2bAVp6R3C85vCYrR6be36n:10.00:5");
    assert_eq!(two["gateValue"], "true");
}

#[tokio::test]
async fn test_exposures_with_environment() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("user-in-layer-holdout-4");

    let statsig =
        create_statsig_with_environment(&specs_adapter, &logging_adapter, Some("dev".to_string()));
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_public");
    let _ = statsig.get_dynamic_config(&user, "test_dynamic_config");
    let layer = statsig.get_layer(&user, "test_layer_in_holdout");
    let _ = layer.get_string("layer_val", String::new());

    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let events = logging_adapter.force_get_received_payloads();
    let event = enforce_object(&events.events[0]);
    let user = enforce_object(&event["user"]);
    assert_eq!(user["statsigEnvironment"]["tier"], "dev");
}

#[tokio::test]
async fn test_exposure_time() {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter();
    let user = StatsigUser::with_user_id("user-in-layer-holdout-4");

    let statsig =
        create_statsig_with_environment(&specs_adapter, &logging_adapter, Some("dev".to_string()));
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_public");
    let _ = statsig.get_dynamic_config(&user, "test_email_config");
    let _ = statsig.get_experiment(&user, "test_experiment_no_targeting");
    let _ = statsig
        .get_layer(&user, "layer_with_many_params")
        .get_string("a_string", String::new());

    let was = Utc::now().timestamp_millis() as u64;
    sleep(Duration::from_millis(100)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events = enforce_array(&payload.events);

    let gate_expo = enforce_object(&events[0]);
    assert_eq!(gate_expo["eventName"], "statsig::gate_exposure");
    assert!(enforce_u64(&gate_expo["time"]) <= was);

    let config_expo = enforce_object(&events[1]);
    assert_eq!(config_expo["eventName"], "statsig::config_exposure");
    assert!(enforce_u64(&config_expo["time"]) <= was);

    let experiment_expo = enforce_object(&events[2]);
    assert_eq!(experiment_expo["eventName"], "statsig::config_exposure");
    assert!(enforce_u64(&experiment_expo["time"]) <= was);

    let layer_expo = enforce_object(&events[3]);
    assert_eq!(layer_expo["eventName"], "statsig::layer_exposure");
    assert!(enforce_u64(&layer_expo["time"]) <= was);
}

fn create_bootrapped_specs_adapter() -> Arc<MockSpecsAdapter> {
    Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"))
}

fn create_bootrapped_specs_adapter_with_sec_expo_number(
    sec_expo_number: u64,
) -> Arc<MockSpecsAdapter> {
    let sdk_configs =
        Map::<String, Value>::from_iter([("sec_expo_number".to_string(), json!(sec_expo_number))]);

    Arc::new(MockSpecsAdapter::with_data_and_sdk_configs(
        "tests/data/eval_proj_dcs.json",
        sdk_configs,
    ))
}

fn create_trowing_specs_adapter() -> Arc<MockSpecsAdapter> {
    Arc::new(MockSpecsAdapter::throwing())
}

fn create_delayed_specs_adapter() -> Arc<MockSpecsAdapter> {
    Arc::new(MockSpecsAdapter::delayed(
        "tests/data/eval_proj_dcs.json",
        100,
    ))
}

fn create_statsig(
    specs_adapter: &Arc<MockSpecsAdapter>,
    logging_adapter: &Arc<MockEventLoggingAdapter>,
) -> Statsig {
    create_statsig_with_sdk_key("secret-shhh", specs_adapter, logging_adapter)
}

fn create_statsig_with_sdk_key(
    sdk_key: &str,
    specs_adapter: &Arc<MockSpecsAdapter>,
    logging_adapter: &Arc<MockEventLoggingAdapter>,
) -> Statsig {
    Statsig::new(
        sdk_key,
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(logging_adapter.clone()),
            ..StatsigOptions::new()
        })),
    )
}

fn create_statsig_with_environment(
    specs_adapter: &Arc<MockSpecsAdapter>,
    logging_adapter: &Arc<MockEventLoggingAdapter>,
    environment: Option<String>,
) -> Statsig {
    Statsig::new(
        "secret-shhh",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(logging_adapter.clone()),
            environment,
            ..StatsigOptions::new()
        })),
    )
}

fn create_statsig_with_experimental_flags(
    sdk_key: &str,
    specs_adapter: &Arc<MockSpecsAdapter>,
    logging_adapter: &Arc<MockEventLoggingAdapter>,
    experimental_flags: HashSet<String>,
) -> Statsig {
    Statsig::new(
        sdk_key,
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(logging_adapter.clone()),
            experimental_flags: Some(experimental_flags),
            ..StatsigOptions::new()
        })),
    )
}

async fn assert_sec_expo_rollout_result(
    sec_expo_number: u64,
    sdk_key: &str,
    should_log_as_primary: bool,
) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(sec_expo_number);
    let user = StatsigUser::with_user_id("a_user_id");

    let statsig = create_statsig_with_experimental_flags(
        sdk_key,
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    let _ = statsig.check_gate(&user, "test_nested_gate_condition");
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<_> = enforce_array(&payload.events)
        .into_iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .collect();

    if should_log_as_primary {
        assert_eq!(events.len(), 3);
        for event in &events {
            assert!(enforce_array(&event["secondaryExposures"]).is_empty());
        }
    } else {
        assert_eq!(events.len(), 1);
        assert_eq!(enforce_array(&events[0]["secondaryExposures"]).len(), 2);
    }
}

async fn assert_secondary_exposures_logged_as_primary_for_eval(
    sdk_key: &str,
    user: StatsigUser,
    evaluate: impl FnOnce(&Statsig, &StatsigUser),
    expected_primary_event: Value,
    expected_secondary_events: Vec<Value>,
) {
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
    let specs_adapter = create_bootrapped_specs_adapter_with_sec_expo_number(1000);

    let statsig = create_statsig_with_experimental_flags(
        sdk_key,
        &specs_adapter,
        &logging_adapter,
        HashSet::from([SEC_EXPO_AS_PRIMARY_FLAG.to_string()]),
    );
    statsig.initialize().await.unwrap();

    evaluate(&statsig, &user);
    sleep(Duration::from_millis(1)).await;
    statsig.flush_events().await;

    let payload = logging_adapter.force_get_received_payloads();
    let events: Vec<Value> = enforce_array(&payload.events)
        .iter()
        .filter(|event| event["eventName"] != "statsig::diagnostics")
        .cloned()
        .collect();

    assert_eq!(events.len(), 1 + expected_secondary_events.len());

    for event in &events {
        assert!(enforce_array(&event["secondaryExposures"]).is_empty());
    }

    assert_json_include!(
        actual: events[0],
        expected: expected_primary_event,
    );

    for (event, expected) in events.iter().skip(1).zip(expected_secondary_events) {
        assert_json_include!(
            actual: event,
            expected: expected,
        );
    }
}

async fn get_reason_from_adapter(logging_adapter: &MockEventLoggingAdapter) -> String {
    let event = logging_adapter.force_get_first_event();
    let metadata = enforce_object(&event["metadata"]);

    enforce_string(&metadata["reason"])
}
