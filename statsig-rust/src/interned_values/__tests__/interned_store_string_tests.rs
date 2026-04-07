use std::{borrow::Cow, time::Instant};

use rusty_fork::rusty_fork_test;
use serde_json::value::RawValue;

use crate::{
    interned_string::{InternedString, InternedStringValue},
    interned_values::InternedStore,
};

const EVAL_PROJ_JSON: &[u8] = include_bytes!("../../../tests/data/eval_proj_dcs.json");
const DEMO_PROJ_PROTO: &[u8] = include_bytes!("../../../tests/data/demo_proj_dcs.pb.br");

#[test]
fn test_interned_string_from_str_ref() {
    let value = String::from_utf8(EVAL_PROJ_JSON.to_vec()).unwrap();
    let start = Instant::now();
    for i in 0..1000 {
        let raw: Cow<'_, RawValue> = serde_json::from_str(&value).unwrap();

        if i == 0 {
            let owned: serde_json::Value = serde_json::from_str(raw.get()).unwrap();
            drop(owned);
        }
        drop(raw);
    }
    let end = Instant::now();
    println!("Time taken: {:?}", end.duration_since(start));

    let start = Instant::now();
    for i in 0..1000 {
        let raw: &RawValue = unsafe { std::mem::transmute::<&str, &RawValue>(value.as_str()) };
        if i == 0 {
            let owned: serde_json::Value = serde_json::from_str(raw.get()).unwrap();
            drop(owned);
        }
    }
    let end = Instant::now();
    println!("Time taken: {:?}", end.duration_since(start));
}

#[test]
fn test_interned_string_non_preloaded() {
    let result = InternedString::from_str_ref("userID");

    assert!(matches!(result.value, InternedStringValue::Pointer(_)));
    assert_eq!(result.as_str(), "userID");
}

rusty_fork_test! {
    #[test]
    fn test_interned_string_preloaded() {
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());

        let key = InternedString::from_str_ref("userID");

        assert!(matches!(key.value, InternedStringValue::Static(_)));
        assert_eq!(key.as_str(), "userID");
    }

    #[test]
    fn test_interned_string_preloaded_multi_payload_json_and_proto() {
        assert!(InternedStore::preload_multi(&[EVAL_PROJ_JSON, DEMO_PROJ_PROTO]).is_ok());

        let eval_key = InternedString::from_str_ref("test_experiment_no_targeting");
        assert!(matches!(eval_key.value, InternedStringValue::Static(_)));
        assert_eq!(eval_key.as_str(), "test_experiment_no_targeting");

        let proto_key = InternedString::from_str_ref("three_groups");
        assert!(matches!(proto_key.value, InternedStringValue::Static(_)));
        assert_eq!(proto_key.as_str(), "three_groups");
    }

    #[test]
    fn test_repeated_calls_to_preload() {
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_err());
    }

    #[test]
    fn test_preloading_across_forks() {
        assert!(InternedStore::preload(EVAL_PROJ_JSON).is_ok());

        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let key = InternedString::from_str_ref("userID");
            assert!(matches!(key.value, InternedStringValue::Static(_)));
            assert_eq!(key.as_str(), "userID");
            std::process::exit(0);
        }

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };
    }

    #[test]
    fn test_non_preloaded_across_forks() {
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let key = InternedString::from_str_ref("userID");
            assert!(matches!(key.value, InternedStringValue::Pointer(_)));
            assert_eq!(key.as_str(), "userID");
            std::process::exit(0);
        }

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };
    }

    #[test]
    fn test_interned_string_dropped() {
        let string = InternedString::from_str_ref("userID");
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        let string2 = InternedString::from_str_ref("userID");
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        drop(string);
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        drop(string2);
        assert_eq!(InternedStore::get_memoized_len().0, 0);
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

            let key = InternedString::from_str_ref("userID");
            assert!(matches!(key.value, InternedStringValue::Static(_)));
            assert_eq!(key.as_str(), "userID");

            std::process::exit(0);
        }

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };
    }

}
