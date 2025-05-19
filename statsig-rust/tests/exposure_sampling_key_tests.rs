use std::collections::HashMap;

use more_asserts::{assert_gt, assert_lt};
use statsig_rust::{
    dyn_value, event_logging::exposure_sampling::ExposureSamplingKey, user::user_data::UserData,
};

fn run_sampling_check(
    sampling_rate: u64,
    user_data: &UserData,
    spec_name_hash: u64,
    rule_id_hash: u64,
) -> bool {
    let sampling_key = ExposureSamplingKey {
        spec_name_hash,
        rule_id_hash,
        user_values_hash: user_data.create_user_values_hash(),
        additional_hash: 0,
    };

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

        if run_sampling_check(100, &user_data, 1, 2) {
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
            custom_ids: Some(HashMap::from([("test".to_string(), dyn_value!(i))])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data, 1, 2) {
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
            custom_ids: Some(HashMap::from([
                ("test".to_string(), dyn_value!(i)),
                ("test2".to_string(), dyn_value!(i.wrapping_mul(i))),
            ])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data, 1, 2) {
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
            custom_ids: Some(HashMap::from([("user_id".to_string(), dyn_value!(i))])),
            ..Default::default()
        };

        if run_sampling_check(100, &user_data, 1, 2) {
            sampled_count += 1;
        }
    }

    assert_gt!(sampled_count, 900);
    assert_lt!(sampled_count, 1100);
}
