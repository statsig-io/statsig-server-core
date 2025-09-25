use std::sync::Arc;

use crate::interned_string::InternedString;

#[test]
fn test_interned_string() {
    let string = InternedString::from_str_ref("test");
    assert_eq!(string.as_str(), "test");
}

#[test]
fn test_interned_string_from_string() {
    let string = InternedString::from_string("test".to_string());
    assert_eq!(string.as_str(), "test");
}

#[test]
fn test_interned_string_from_bool() {
    let string = InternedString::from_bool(true);
    assert_eq!(string.as_str(), "true");
}

#[test]
fn test_comparison_works() {
    let string = InternedString::from_str_ref("test");
    let string2 = InternedString::from_str_ref("test");
    assert_eq!(string, string2);
}

#[test]
fn test_comparison_works_with_different_strings() {
    let string = InternedString::from_str_ref("test");
    let string2 = InternedString::from_str_ref("test2");
    assert_ne!(string, string2);
}

#[test]
fn test_comparison_ignores_hash() {
    let string = InternedString {
        hash: 1,
        value: Arc::new("test".to_string()),
    };
    let string2 = InternedString {
        hash: 2,
        value: Arc::new("test".to_string()),
    };
    assert_eq!(string, string2);
}
