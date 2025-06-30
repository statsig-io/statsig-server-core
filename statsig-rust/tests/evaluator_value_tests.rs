use std::collections::HashMap;

use serde_json::json;
use statsig_rust::{dyn_value, test_only_make_eval_value};

#[test]
fn test_own_bool_comparison() {
    let left = test_only_make_eval_value!(true);
    let right = test_only_make_eval_value!(false);
    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_own_int_comparison() {
    let left = test_only_make_eval_value!(1);
    let right = test_only_make_eval_value!(2);
    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_own_float_comparison() {
    let left = test_only_make_eval_value!(1.0_f64);
    let right = test_only_make_eval_value!(2.0_f64);
    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_own_string_comparison() {
    let left = test_only_make_eval_value!("apple");
    let right = test_only_make_eval_value!("banana");
    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_own_array_comparison() {
    let left = test_only_make_eval_value!([1, 2, 3]);
    let right = test_only_make_eval_value!([4, 5]);

    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_own_object_comparison() {
    let left = test_only_make_eval_value!(HashMap::from([("1".to_string(), 2)]));
    let right = test_only_make_eval_value!(HashMap::from([("3".to_string(), 4)]));

    println!("{right:?}");

    assert_eq!(left, left);
    assert_ne!(left, right);
}

#[test]
fn test_dyn_value_bool_comparison() {
    let ev_true = test_only_make_eval_value!(true);

    let dv_true = dyn_value!(true);
    let dv_false = dyn_value!(false);

    assert!(ev_true.is_equal_to_dynamic_value(&dv_true));
    assert!(!ev_true.is_equal_to_dynamic_value(&dv_false));
}

#[test]
fn test_dyn_value_int_comparison() {
    let ev_1 = test_only_make_eval_value!(1);

    let dv_1 = dyn_value!(1);
    let dv_2 = dyn_value!(2);

    assert!(ev_1.is_equal_to_dynamic_value(&dv_1));
    assert!(!ev_1.is_equal_to_dynamic_value(&dv_2));
}

#[test]
fn test_dyn_value_float_comparison() {
    let ev_1 = test_only_make_eval_value!(1.0_f64);

    let dv_1 = dyn_value!(1.0_f64);
    let dv_2 = dyn_value!(2.0_f64);

    assert!(ev_1.is_equal_to_dynamic_value(&dv_1));
    assert!(!ev_1.is_equal_to_dynamic_value(&dv_2));
}

#[test]
fn test_dyn_value_string_comparison() {
    let ev_apple = test_only_make_eval_value!("apple");

    let dv_apple = dyn_value!("apple");
    let dv_banana = dyn_value!("banana");

    assert!(ev_apple.is_equal_to_dynamic_value(&dv_apple));
    assert!(!ev_apple.is_equal_to_dynamic_value(&dv_banana));
}

#[test]
fn test_dyn_value_array_comparison() {
    let ev_array = test_only_make_eval_value!([1, 2, 3]);

    let dv_array_1 = dyn_value!(json!(vec![1, 2, 3]));
    let dv_array_2 = dyn_value!(json!(vec![4, 5]));

    assert!(ev_array.is_equal_to_dynamic_value(&dv_array_1));
    assert!(!ev_array.is_equal_to_dynamic_value(&dv_array_2));
}

#[test]
fn test_dyn_value_object_comparison() {
    let ev_object = test_only_make_eval_value!(HashMap::from([("1".to_string(), 2)]));
    let dv_object_1 = dyn_value!(json!({"1": 2}));
    let dv_object_2 = dyn_value!(json!({"3": 4}));

    assert!(ev_object.is_equal_to_dynamic_value(&dv_object_1));
    assert!(!ev_object.is_equal_to_dynamic_value(&dv_object_2));
}
