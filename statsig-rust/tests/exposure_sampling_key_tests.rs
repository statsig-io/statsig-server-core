use std::collections::HashSet;

use more_asserts::{assert_gt, assert_lt};
use serde_json::json;
use statsig_rust::{
    dyn_value, evaluation::evaluation_types::BaseEvaluation,
    event_logging::exposure_sampling::ExposureSamplingKey, user::user_data::UserData,
    StatsigUserDataMap,
};

fn run_sampling_check(sampling_rate: u64, user_data: &UserData) -> bool {
    let sampling_key = ExposureSamplingKey::new(None, user_data, 0, None);

    sampling_key.is_sampled(Some(sampling_rate))
}

#[test]
fn test_is_sampled_user_id_only() {
    let mut sampled_count = 0;

    for i in 0..100_000u64 {
        let user_data = UserData {
            user_id: Some(dyn_value!(i)),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data) {
            sampled_count += 1;
        }
    }

    assert_gt!(sampled_count, 900);
    assert_lt!(sampled_count, 1100);
}

#[test]
fn test_is_sampled_single_custom_id_only() {
    let mut sampled_count = 0;

    for i in 0..100_000u64 {
        let user_data = UserData {
            custom_ids: Some(StatsigUserDataMap::from([(
                "test".to_string(),
                dyn_value!(i),
            )])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data) {
            sampled_count += 1;
        }
    }

    assert_gt!(sampled_count, 900);
    assert_lt!(sampled_count, 1100);
}

#[test]
fn test_is_sampled_multiple_custom_ids_only() {
    let mut sampled_count = 0;

    for i in 0..100_000u64 {
        let user_data = UserData {
            custom_ids: Some(StatsigUserDataMap::from([
                ("test".to_string(), dyn_value!(i)),
                ("test2".to_string(), dyn_value!(i.wrapping_mul(i))),
            ])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data) {
            sampled_count += 1;
        }
    }

    assert_gt!(sampled_count, 900);
    assert_lt!(sampled_count, 1100);
}

#[test]
fn test_duplicate_unit_ids() {
    let mut sampled_count = 0;

    for i in 0..100_000u64 {
        let user_data = UserData {
            user_id: Some(dyn_value!(i)),
            custom_ids: Some(StatsigUserDataMap::from([(
                "user_id".to_string(),
                dyn_value!(i),
            )])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data) {
            sampled_count += 1;
        }
    }

    assert_gt!(sampled_count, 900);
    assert_lt!(sampled_count, 1100);
}

#[test]
fn test_dedupe_key_is_stable_for_repeated_same_user_input() {
    let user_data = UserData {
        user_id: Some(dyn_value!("user-1")),
        custom_ids: Some(StatsigUserDataMap::from([
            ("companyID".to_string(), dyn_value!("company-a")),
            ("teamID".to_string(), dyn_value!(99)),
        ])),
        ..Default::default()
    };

    let key_a = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );
    let key_b = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );

    assert_eq!(key_a, key_b);

    let mut dedupe = HashSet::new();
    assert!(dedupe.insert(key_a));
    assert!(!dedupe.insert(key_b));
}

#[test]
fn test_dedupe_key_changes_when_unit_id_changes_for_experiment_id_type() {
    let mut user_data = UserData {
        user_id: Some(dyn_value!("user-1")),
        custom_ids: Some(StatsigUserDataMap::from([(
            "companyID".to_string(),
            dyn_value!("company-a"),
        )])),
        ..Default::default()
    };

    let key_a = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );

    let mut updated_custom_ids = user_data.custom_ids.take().unwrap();
    updated_custom_ids.insert("companyID".to_string(), dyn_value!("company-b"));
    user_data.custom_ids = Some(updated_custom_ids);

    let key_b = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );

    assert_ne!(key_a, key_b);
}

#[test]
fn test_dedupe_key_changes_when_custom_ids_hash_scope_changes() {
    let mut user_data = UserData {
        user_id: Some(dyn_value!("user-1")),
        ..Default::default()
    };

    let key_no_custom = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );

    user_data.custom_ids = Some(StatsigUserDataMap::from([
        ("companyID".to_string(), dyn_value!("company-a")),
        ("teamID".to_string(), dyn_value!("team-a")),
    ]));

    let key_with_custom = ExposureSamplingKey::new(
        Some(&fake_evaluation("spec_a", "rule_a")),
        &user_data,
        7,
        Some("companyID"),
    );

    assert_ne!(key_no_custom, key_with_custom);
}

fn fake_evaluation(name: &str, rule_id: &str) -> BaseEvaluation {
    serde_json::from_value(json!({
        "name": name,
        "rule_id": rule_id,
        "secondary_exposures": [],
    }))
    .expect("failed to build fake base evaluation")
}
