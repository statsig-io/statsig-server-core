use std::sync::{Mutex, MutexGuard};
use std::thread;

use lazy_static::lazy_static;
use sigstat::instance_store::{OPTIONS_INSTANCES, STATSIG_INSTANCES};
use sigstat::{instance_store::USER_INSTANCES, Statsig, StatsigOptions, StatsigUser};
use std::collections::HashSet;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    static ref USER: StatsigUser = StatsigUser::with_user_id("a-user".to_string());
}

fn get_test_lock() -> MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap();

    USER_INSTANCES.release_all();
    STATSIG_INSTANCES.release_all();
    OPTIONS_INSTANCES.release_all();

    guard
}

#[test]
fn test_instance_id_prefix() {
    let _lock = get_test_lock();

    assert!(USER_INSTANCES
        .add(USER.clone())
        .unwrap()
        .starts_with("usr_"));
    assert!(STATSIG_INSTANCES
        .add(Statsig::new("", None))
        .unwrap()
        .starts_with("stsg_"));
    assert!(OPTIONS_INSTANCES
        .add(StatsigOptions::new())
        .unwrap()
        .starts_with("opts_"));
}

#[test]
fn test_creation_limit() {
    let _lock = get_test_lock();

    for _ in 0..100_000 {
        let inst_id = USER_INSTANCES.add(USER.clone());
        assert!(inst_id.is_some());
    }

    let inst_id = USER_INSTANCES.add(USER.clone());
    assert!(inst_id.is_none());
}

#[test]
fn test_removing_resets_limit() {
    let _lock = get_test_lock();

    for _ in 0..100_000 {
        let inst_id = USER_INSTANCES.add(USER.clone());
        USER_INSTANCES.release(inst_id.unwrap());
    }

    let inst_id = USER_INSTANCES.add(USER.clone());
    assert!(inst_id.is_some());
}

#[test]
fn test_unique_id_generation() {
    let _lock = get_test_lock();

    let mut ids = HashSet::new();
    for _ in 0..1000 {
        let id = USER_INSTANCES.add(USER.clone()).unwrap();
        ids.insert(id.clone());
    }

    assert!(ids.len() == 1000);
}

#[test]
fn test_concurrent_access() {
    let _lock = get_test_lock();

    let threads: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..1000 {
                    let id = USER_INSTANCES.add(USER.clone()).unwrap();

                    assert!(USER_INSTANCES.get(id.clone()).is_some());

                    USER_INSTANCES.release(id);
                }
            })
        })
        .collect();

    for thread in threads {
        thread.join().unwrap();
    }

    assert!(USER_INSTANCES.add(USER.clone()).is_some());
}

#[test]
fn test_invalid_id_handling() {
    let _lock = get_test_lock();

    let inst_id = USER_INSTANCES.add(USER.clone());

    assert!(USER_INSTANCES.get("invalid_id".to_string()).is_none());
    assert!(STATSIG_INSTANCES.get(inst_id.unwrap()).is_none());

    USER_INSTANCES.release("invalid_id".to_string()); // Should not panic
}

#[test]
fn test_optional_get() {
    let _lock = get_test_lock();

    let inst_id = USER_INSTANCES.add(USER.clone());
    assert!(USER_INSTANCES.optional_get(inst_id.clone()).is_some());
    assert!(USER_INSTANCES.optional_get(None).is_none());
}

#[test]
fn test_correct_instance_type_handling() {
    let _lock = get_test_lock();

    let user_id = USER_INSTANCES.add(USER.clone()).unwrap();
    let statsig_id = STATSIG_INSTANCES.add(Statsig::new("", None)).unwrap();
    let options_id = OPTIONS_INSTANCES.add(StatsigOptions::new()).unwrap();

    assert!(USER_INSTANCES.get(user_id.clone()).is_some());
    assert!(USER_INSTANCES.get(statsig_id.clone()).is_none());
    assert!(USER_INSTANCES.get(options_id.clone()).is_none());

    assert!(STATSIG_INSTANCES.get(statsig_id.clone()).is_some());
    assert!(STATSIG_INSTANCES.get(user_id.clone()).is_none());
    assert!(STATSIG_INSTANCES.get(options_id.clone()).is_none());

    assert!(OPTIONS_INSTANCES.get(options_id.clone()).is_some());
    assert!(OPTIONS_INSTANCES.get(user_id.clone()).is_none());
    assert!(OPTIONS_INSTANCES.get(statsig_id.clone()).is_none());

    USER_INSTANCES.release(user_id);
    STATSIG_INSTANCES.release(statsig_id);
    OPTIONS_INSTANCES.release(options_id);
}
