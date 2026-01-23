use std::collections::HashMap;

use rusty_fork::rusty_fork_test;

use crate::{
    evaluation::dynamic_returnable::DynamicReturnableValue, interned_values::InternedStore,
    DynamicReturnable,
};

#[test]
fn test_interned_returnable_non_bootstrapped() {
    let bool_res = DynamicReturnable::from_bool(true);
    assert!(matches!(bool_res.value, DynamicReturnableValue::Bool(_)));

    let null_res = DynamicReturnable::empty();
    assert!(matches!(null_res.value, DynamicReturnableValue::Null));

    let json_res = DynamicReturnable::from_map(HashMap::from([(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    )]));
    assert!(matches!(
        json_res.value,
        DynamicReturnableValue::JsonPointer(_)
    ));
}

rusty_fork_test! {
    #[test]
    fn test_interned_returnable_bootstrapped() {
        // test_experiment_no_targeting.rules[1]["returnValue"] -> {"value":"control"}
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::bootstrap(data).is_ok());

        let bool_res = DynamicReturnable::from_bool(true);
        assert!(matches!(bool_res.value, DynamicReturnableValue::Bool(_)));

        let null_res = DynamicReturnable::empty();
        assert!(matches!(null_res.value, DynamicReturnableValue::Null));

        let json_res = DynamicReturnable::from_map(HashMap::from([(
            "value".to_string(),
            serde_json::Value::String("control".to_string()),
        )]));
        assert!(matches!(json_res.value, DynamicReturnableValue::JsonStatic(_)));

        let again = json_res.get_json().unwrap();
        assert_eq!(again, HashMap::from([(
            "value".to_string(),
            serde_json::Value::String("control".to_string()),
        )]));
    }

    #[test]
    fn test_interned_returnable_dropped() {
        let returnable = DynamicReturnable::from_map(HashMap::from([(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        )]));
        assert_eq!(InternedStore::get_memoized_len().1, 1);

        let returnable2 = DynamicReturnable::from_map(HashMap::from([(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        )]));
        assert_eq!(InternedStore::get_memoized_len().1, 1);

        drop(returnable);
        assert_eq!(InternedStore::get_memoized_len().1, 1);

        drop(returnable2);
        assert_eq!(InternedStore::get_memoized_len().1, 0);
    }
}
