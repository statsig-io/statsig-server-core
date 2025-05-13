use more_asserts::assert_gt;
use serde_json::json;
use statsig_rust::evaluation::dynamic_string::DynamicString;

#[test]
fn test_creation_from_string() {
    let dyn_str = DynamicString::from("aB1!".to_string());

    assert_eq!(dyn_str.value, "aB1!");
    assert_eq!(dyn_str.lowercased_value, "ab1!");
}

#[test]
fn test_creation_from_json() {
    let dyn_str = DynamicString::from(json!("aB1!"));

    assert_eq!(dyn_str.value, "aB1!");
    assert_eq!(dyn_str.lowercased_value, "ab1!");
}

#[test]
fn test_comparison_against_str() {
    let dyn_str = DynamicString::from("a_string".to_string());

    assert_eq!(dyn_str, "a_string");
}

#[test]
fn test_comparison_against_string() {
    let dyn_str = DynamicString::from("a_string".to_string());

    assert_eq!(dyn_str, "a_string".to_string());
}

#[test]
fn test_comparison_against_string_ref() {
    let dyn_str = DynamicString::from("a_string".to_string());

    let a_string = "a_string".to_string();
    assert_eq!(dyn_str, &a_string);
}

#[test]
fn test_hash_value_is_consistent() {
    let s = "a_string".to_string();
    let mut hash = DynamicString::from(s.clone()).hash_value;

    for _ in 0..10000 {
        let new_hash = DynamicString::from(s.clone()).hash_value;
        assert_eq!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}

#[test]
fn test_hash_value_is_changing() {
    let mut hash = DynamicString::from("a_string".to_string()).hash_value;

    for i in 0..10000 {
        let new_hash = DynamicString::from(i.to_string()).hash_value;
        assert_ne!(hash, new_hash);
        assert_gt!(new_hash, 0);
        hash = new_hash;
    }
}
