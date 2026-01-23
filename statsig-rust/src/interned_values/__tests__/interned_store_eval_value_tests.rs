use rusty_fork::rusty_fork_test;

use crate::{
    evaluation::evaluator_value::{EvaluatorValue, EvaluatorValueInner},
    interned_string::InternedString,
    interned_values::InternedStore,
};

#[test]
fn test_interned_eval_value_non_bootstrapped() {
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
    fn test_interned_eval_value_bootstrapped() {
        // condition_map["7718260"]["targetValue"] -> ["@statsig","@stotseg"]
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::bootstrap(data).is_ok());

        let eval_value = EvaluatorValue::from_json_value(serde_json::Value::Array(vec![
            json_string("@statsig"),
            json_string("@stotseg"),
        ]));

        assert!(matches!(eval_value.inner, EvaluatorValueInner::Static(_)));
        let actual = match &eval_value.inner {
            EvaluatorValueInner::Static(value) => value.array_value.as_ref().unwrap(),
            _ => panic!("Expected static"),
        };

        let keys = actual.keys().collect::<Vec<_>>();
        println!("keys: {:?}", keys);

        assert_eq!(keys[0], &InternedString::from_str_ref("@statsig"));
        assert_eq!(keys[1], &InternedString::from_str_ref("@stotseg"));
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
    fn test_bootstrapped_compiles_str_matches_conditions() {
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::bootstrap(data).is_ok());

        // condition_map["998446160"]["targetValue"] -> "@.*mail"
        let eval_value = EvaluatorValue::from_json_value(json_string("@.*mail"));
        let actual = match &eval_value.inner {
            EvaluatorValueInner::Static(value) => value,
            _ => panic!("Expected static"),
        };

        assert!(actual.regex_value.is_some(), "Should compile regex for str_matches conditions");
    }

    #[test]
    fn test_bootstrapped_does_not_compile_non_str_matches_conditions() {
        let data = include_bytes!("../../../tests/data/eval_proj_dcs.json");
        assert!(InternedStore::bootstrap(data).is_ok());

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
