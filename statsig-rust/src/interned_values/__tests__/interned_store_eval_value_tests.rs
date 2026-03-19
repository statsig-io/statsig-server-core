use rusty_fork::rusty_fork_test;

use crate::{
    evaluation::evaluator_value::{EvaluatorValue, EvaluatorValueInner},
    interned_string::{InternedString, InternedStringValue},
    interned_values::InternedStore,
};

const EVAL_PROJ_JSON: &[u8] = include_bytes!("../../../tests/data/eval_proj_dcs.json");
const DEMO_PROJ_PROTO: &[u8] = include_bytes!("../../../tests/data/demo_proj_dcs.pb.br");

#[test]
fn test_interned_eval_value_non_preloaded() {
    let eval_value = EvaluatorValue::from_json_value(serde_json::Value::Bool(true));
    assert!(matches!(eval_value.inner, EvaluatorValueInner::Pointer(_)));

    let actual = match &eval_value.inner {
        EvaluatorValueInner::Pointer(value) => value,
        _ => panic!("Expected pointer"),
    };

    assert_eq!(actual.bool_value, Some(true));
}

rusty_fork_test! {
    #[test]
    fn test_interned_eval_value_preloaded() {
        // condition_map["7718260"]["targetValue"] -> ["@statsig","@stotseg"]
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());

        let eval_value = EvaluatorValue::from_json_value(serde_json::Value::Array(vec![
            json_string("@statsig"),
            json_string("@stotseg"),
        ]));

        assert!(matches!(eval_value.inner, EvaluatorValueInner::Static(_)));
        let actual = match &eval_value.inner {
            EvaluatorValueInner::Static(value) => value.array_value.as_ref().unwrap(),
            _ => panic!("Expected static"),
        };

        assert!(actual.contains_key(&InternedString::from_str_ref("@statsig")));
        assert!(actual.contains_key(&InternedString::from_str_ref("@stotseg")));
    }

    #[test]
    fn test_interned_eval_value_preloaded_multi_payload_json_and_proto() {
        assert!(InternedStore::preload_multi(&[EVAL_PROJ_JSON, DEMO_PROJ_PROTO]).is_ok());

        let demo_target_value = EvaluatorValue::from_json_value(serde_json::Value::Array(vec![
            json_string("1"),
            json_string("2"),
            json_string("3"),
            json_string("4"),
            json_string("5"),
        ]));
        assert!(matches!(demo_target_value.inner, EvaluatorValueInner::Static(_)));

        let from_proto_payload_key = InternedString::from_str_ref("three_groups");
        assert!(matches!(from_proto_payload_key.value, InternedStringValue::Static(_)));
        assert_eq!(from_proto_payload_key.as_str(), "three_groups");

        let from_json_payload_key = InternedString::from_str_ref("test_experiment_no_targeting");
        assert!(matches!(from_json_payload_key.value, InternedStringValue::Static(_)));
        assert_eq!(from_json_payload_key.as_str(), "test_experiment_no_targeting");
    }


    #[test]
    fn test_interned_eval_value_dropped() {
        let eval_value = EvaluatorValue::from_json_value(json_string("foo"));
        assert_eq!(InternedStore::get_memoized_len().2, 1);

        let eval_value2 = EvaluatorValue::from_json_value(json_string("foo"));
        assert_eq!(InternedStore::get_memoized_len().2, 1);

        drop(eval_value);
        assert_eq!(InternedStore::get_memoized_len().2, 1);

        drop(eval_value2);
        assert_eq!(InternedStore::get_memoized_len().2, 0);
    }

    #[test]
    fn test_preloaded_compiles_str_matches_conditions() {
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::preload(data).is_ok());

        // condition_map["998446160"]["targetValue"] -> "@.*mail"
        let eval_value = EvaluatorValue::from_json_value(json_string("@.*mail"));
        let actual = match &eval_value.inner {
            EvaluatorValueInner::Static(value) => value,
            _ => panic!("Expected static"),
        };

        assert!(actual.regex_value.is_some(), "Should compile regex for str_matches conditions");
    }

    #[test]
    fn test_preloaded_does_not_compile_non_str_matches_conditions() {
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::preload(data).is_ok());

        // condition_map["392358526"]["targetValue"] -> "test_email"
        let eval_value = EvaluatorValue::from_json_value(json_string("test_email"));
        let actual = match &eval_value.inner {
            EvaluatorValueInner::Static(value) => value,
            _ => panic!("Expected static"),
        };

        assert!(actual.regex_value.is_none(), "Should not compile regex for non str_matches conditions");
    }
}

fn json_string(value: &str) -> serde_json::Value {
    serde_json::Value::String(value.to_string())
}
