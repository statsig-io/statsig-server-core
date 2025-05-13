use std::sync::Arc;

use more_asserts::assert_gt;
use statsig_rust::event_logging::exposable_string::ExposableString;

#[test]
fn test_clone_only_impacts_inner() {
    let orig = Arc::new("a_string".to_string());
    let expo_str = ExposableString::from_arc(orig.clone());

    assert_eq!(Arc::strong_count(&orig), 2);

    let expo_str_clone = expo_str.clone();
    assert_eq!(Arc::strong_count(&orig), 3);
    assert_eq!(expo_str_clone.as_str(), "a_string");
}

#[test]
fn test_as_ref_does_not_clone() {
    let orig = Arc::new("a_string".to_string());
    let expo_str = ExposableString::from_arc(orig.clone());
    let expo_str_ref = expo_str.as_str();

    assert_eq!(Arc::strong_count(&orig), 2);
    assert_eq!(expo_str_ref, "a_string");
}

#[test]
fn test_extracting_the_inner_value() {
    let expo_str = ExposableString::new("a_string".to_string());
    let owned_string = expo_str.unperformant_to_string();

    assert_eq!(owned_string, "a_string".to_string());
}

#[test]
fn test_exposable_string_hash_value_is_consistent() {
    let s = "a_string".to_string();
    let mut hash = ExposableString::new(s.clone()).hash_value;

    for _ in 0..10000 {
        let new_hash = ExposableString::new(s.clone()).hash_value;
        assert_eq!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}

#[test]
fn test_exposable_string_hash_value_is_changing() {
    let s = "a_string".to_string();
    let mut hash = ExposableString::new(s.clone()).hash_value;

    for i in 0..10000 {
        let new_hash = ExposableString::new(i.to_string()).hash_value;
        assert_ne!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}
