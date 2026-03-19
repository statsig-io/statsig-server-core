use std::collections::HashMap;

use rusty_fork::rusty_fork_test;

use crate::{
    evaluation::dynamic_returnable::DynamicReturnableValue,
    interned_string::{InternedString, InternedStringValue},
    interned_values::InternedStore,
    DynamicReturnable,
};

const EVAL_PROJ_JSON: &[u8] = include_bytes!("../../../tests/data/eval_proj_dcs.json");
const DEMO_PROJ_PROTO: &[u8] = include_bytes!("../../../tests/data/demo_proj_dcs.pb.br");

#[test]
fn test_interned_returnable_non_preloaded() {
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
    fn test_interned_returnable_preloaded() {
        // test_experiment_no_targeting.rules[1]["returnValue"] -> {"value":"control"}
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());

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
    fn test_interned_returnable_preloaded_multi_payload_json_and_proto() {
        assert!(InternedStore::preload_multi(&[EVAL_PROJ_JSON, DEMO_PROJ_PROTO]).is_ok());

        let eval_key = InternedString::from_str_ref("test_experiment_no_targeting");
        assert!(matches!(eval_key.value, InternedStringValue::Static(_)));
        assert_eq!(eval_key.as_str(), "test_experiment_no_targeting");

        let proto_key = InternedString::from_str_ref("three_groups");
        assert!(matches!(proto_key.value, InternedStringValue::Static(_)));
        assert_eq!(proto_key.as_str(), "three_groups");
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
