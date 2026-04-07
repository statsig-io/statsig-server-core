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
    fn test_dynamic_returnable_value_eq_pointer_and_static_json_variants() {
        let value = HashMap::from([(
            "value".to_string(),
            serde_json::Value::String("control".to_string()),
        )]);
        let pointer = DynamicReturnable::from_map(value.clone());
        assert!(matches!(pointer.value, DynamicReturnableValue::JsonPointer(_)));

        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());
        let static_value = DynamicReturnable::from_map(value.clone());
        assert!(matches!(static_value.value, DynamicReturnableValue::JsonStatic(_)));

        assert_eq!(&pointer.value, &static_value.value);
    }

    #[test]
    fn test_dynamic_returnable_value_eq_pointer_and_archived_json_variants() {
        let value = HashMap::from([(
            "value".to_string(),
            serde_json::Value::String("control".to_string()),
        )]);
        let pointer = DynamicReturnable::from_map(value.clone());
        assert!(matches!(pointer.value, DynamicReturnableValue::JsonPointer(_)));

        let mmap_file = tempfile::NamedTempFile::new().unwrap();
        let mmap_path = mmap_file.path().to_str().unwrap();
        assert!(InternedStore::write_mmap_data(&[EVAL_PROJ_JSON], mmap_path).is_ok());
        assert!(InternedStore::preload_mmap(mmap_path).is_ok());

        let archived = DynamicReturnable::from_map(value);
        assert!(matches!(
            archived.value,
            DynamicReturnableValue::JsonArchived(_)
        ));

        assert_eq!(&pointer.value, &archived.value);
    }

    #[test]
    fn test_dynamic_returnable_value_eq_distinguishes_non_matching_variants() {
        let null_value = DynamicReturnable::empty();
        let true_value = DynamicReturnable::from_bool(true);
        let false_value = DynamicReturnable::from_bool(false);
        let json_value = DynamicReturnable::from_map(HashMap::from([(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        )]));

        assert_eq!(&null_value.value, &DynamicReturnableValue::Null);
        assert_eq!(&true_value.value, &DynamicReturnableValue::Bool(true));
        assert_ne!(&true_value.value, &false_value.value);
        assert_ne!(&null_value.value, &true_value.value);
        assert_ne!(&null_value.value, &json_value.value);
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

    #[test]
    fn test_preloading_mmap_across_forks() {
        let path = "/tmp/statsig-rust-test-mmap.bin";
        if std::fs::File::open(path).is_ok() {
            std::fs::remove_file(path).unwrap();
        }

        assert!(InternedStore::write_mmap_data(&[EVAL_PROJ_JSON], path).is_ok());

        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let result = InternedStore::preload_mmap(path);
            assert!(result.is_ok());

            let json_res = DynamicReturnable::from_map(HashMap::from([(
                "value".to_string(),
                serde_json::Value::String("control".to_string()),
            )]));
            assert!(matches!(
                json_res.value,
                DynamicReturnableValue::JsonArchived(_)
            ));

            std::process::exit(0);
        }

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };
    }

}
