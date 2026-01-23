use rusty_fork::rusty_fork_test;

use crate::{
    interned_string::{InternedString, InternedStringValue},
    interned_values::InternedStore,
};

#[test]
fn test_interned_string_non_bootstrapped() {
    let result = InternedString::from_str_ref("test");

    assert!(matches!(result.value, InternedStringValue::Pointer(_)));
    assert_eq!(result.as_str(), "test");
}

rusty_fork_test! {
    #[test]
    fn test_interned_string_bootstrapped() {
        let data = b"{\"test\":\"value\"}";
        assert!(InternedStore::bootstrap(data).is_ok());

        let key = InternedString::from_str_ref("test");

        assert!(matches!(key.value, InternedStringValue::Static(_)));
        assert_eq!(key.as_str(), "test");

        let value = InternedString::from_str_ref("value");
        assert!(matches!(value.value, InternedStringValue::Static(_)));
        assert_eq!(value.as_str(), "value");
    }

    #[test]
    fn test_repeated_calls_to_bootstrap() {
        let data = b"{\"test\":\"value\"}";
        assert!(InternedStore::bootstrap(data).is_ok());
        assert!(InternedStore::bootstrap(data).is_err());
    }

    #[test]
    fn test_bootstrapping_across_forks() {
        let data = b"{\"test\":\"value\"}";
        assert!(InternedStore::bootstrap(data).is_ok());

        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let key = InternedString::from_str_ref("test");
            assert!(matches!(key.value, InternedStringValue::Static(_)));
            assert_eq!(key.as_str(), "test");
            std::process::exit(0);
        }

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };
    }

    #[test]
    fn test_non_bootstrapped_across_forks() {
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let key = InternedString::from_str_ref("test");
            assert!(matches!(key.value, InternedStringValue::Pointer(_)));
            assert_eq!(key.as_str(), "test");
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
        let string = InternedString::from_str_ref("test");
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        let string2 = InternedString::from_str_ref("test");
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        drop(string);
        assert_eq!(InternedStore::get_memoized_len().0, 1);

        drop(string2);
        assert_eq!(InternedStore::get_memoized_len().0, 0);
    }
}
