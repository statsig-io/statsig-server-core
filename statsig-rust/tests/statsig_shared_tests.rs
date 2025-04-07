mod utils;

use lazy_static::lazy_static;
use statsig_rust::{Statsig, StatsigUser};
use std::sync::{Arc, Mutex, MutexGuard};

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn setup() -> MutexGuard<'static, ()> {
    let lock = TEST_MUTEX.lock().unwrap();
    Statsig::remove_shared();
    lock
}

#[test]
fn test_create_shared_statsig() {
    let _lock = setup();

    let statsig = Statsig::new_shared("secret-key", None).unwrap();
    let shared = Statsig::shared();

    assert!(Arc::ptr_eq(&statsig, &shared));
}

#[test]
fn test_shared_vs_individual_instance() {
    let _lock = setup();

    let statsig = Statsig::new_shared("secret-key", None).unwrap();
    let statsig_individual = Statsig::new("secret-key", None);

    let statsig_ptr = Arc::as_ptr(&statsig);
    let statsig_individual_ptr = &statsig_individual as *const Statsig;

    assert_ne!(statsig_ptr, statsig_individual_ptr);
}

#[test]
fn test_calling_shared_before_creation() {
    let _lock = setup();

    let shared1 = Statsig::shared();
    let shared2 = Statsig::shared();

    assert!(!Arc::ptr_eq(&shared1, &shared2));
}

#[test]
fn test_calling_shared_before_and_after_creation() {
    let _lock = setup();

    let shared1 = Statsig::shared();
    let statsig = Statsig::new_shared("secret-key", None).unwrap();
    let shared2 = Statsig::shared();

    assert!(!Arc::ptr_eq(&shared1, &shared2));
    assert!(Arc::ptr_eq(&statsig, &shared2));
}

#[test]
fn test_creating_consecutive_shared() {
    let _lock = setup();

    let _ = Statsig::new_shared("secret-key", None).unwrap();
    let statsig2 = Statsig::new_shared("secret-key", None);

    assert!(statsig2.is_err());
}

#[test]
fn test_calling_functions_on_shared() {
    let _lock = setup();

    let _ = Statsig::new_shared("secret-key", None);
    let user = StatsigUser::with_user_id("a-user".to_string());

    assert!(!Statsig::shared().check_gate(&user, "not-found"));
}

#[test]
fn test_checking_if_shared_instance_does_not_exist() {
    let _lock = setup();

    assert!(!Statsig::has_shared_instance());
}

#[test]
fn test_checking_if_shared_instance_exists() {
    let _lock = setup();
    let _ = Statsig::new_shared("secret-key", None);

    assert!(Statsig::has_shared_instance());
}
