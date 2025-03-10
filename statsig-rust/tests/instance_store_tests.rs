use std::sync::{Mutex, MutexGuard};
use std::thread;

use lazy_static::lazy_static;
use statsig_rust::{instance_store::INST_STORE, Statsig, StatsigOptions, StatsigUser};
use std::collections::HashSet;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    static ref USER: StatsigUser = StatsigUser::with_user_id("a-user".to_string());
}

fn get_test_lock() -> MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap();

    INST_STORE.remove_all();

    guard
}

#[test]
fn test_instance_id_prefix() {
    let _lock = get_test_lock();

    assert!(INST_STORE.add(USER.clone()).unwrap().starts_with("usr_"));
    assert!(INST_STORE
        .add(Statsig::new("", None))
        .unwrap()
        .starts_with("stsg_"));
    assert!(INST_STORE
        .add(StatsigOptions::new())
        .unwrap()
        .starts_with("opts_"));
}

#[test]
fn test_creation_limit() {
    let _lock = get_test_lock();

    for _ in 0..400_000 {
        let inst_id = INST_STORE.add(USER.clone());
        assert!(inst_id.is_some());
    }

    let inst_id = INST_STORE.add(USER.clone());
    assert!(inst_id.is_none());
}

#[test]
fn test_removing_resets_limit() {
    let _lock = get_test_lock();

    for _ in 0..400_000 {
        let inst_id = INST_STORE.add(USER.clone());
        INST_STORE.remove(&inst_id.unwrap());
    }

    let inst_id = INST_STORE.add(USER.clone());
    assert!(inst_id.is_some());
}

#[test]
fn test_unique_id_generation() {
    let _lock = get_test_lock();

    let mut ids = HashSet::new();
    for _ in 0..1000 {
        let id = INST_STORE.add(USER.clone()).unwrap();
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
                    let id = INST_STORE.add(USER.clone()).unwrap();

                    assert!(INST_STORE.get::<StatsigUser>(&id).is_some());

                    INST_STORE.remove(&id);
                }
            })
        })
        .collect();

    for thread in threads {
        thread.join().unwrap();
    }

    assert!(INST_STORE.add(USER.clone()).is_some());
}

#[test]
fn test_invalid_id_handling() {
    let _lock = get_test_lock();

    _ = INST_STORE.add(USER.clone());

    assert!(INST_STORE.get::<StatsigUser>("invalid_id").is_none());

    INST_STORE.remove("invalid_id"); // Should not panic
}

#[test]
fn test_optional_get() {
    let _lock = get_test_lock();

    let inst_id = INST_STORE.add(USER.clone());
    assert!(INST_STORE
        .get_with_optional_id::<StatsigUser>(inst_id.as_ref())
        .is_some());
    assert!(INST_STORE
        .get_with_optional_id::<StatsigUser>(None)
        .is_none());
}

#[test]
fn test_correct_instance_type_handling() {
    let _lock = get_test_lock();

    let user_id = INST_STORE.add(USER.clone()).unwrap();
    let statsig_id = INST_STORE.add(Statsig::new("", None)).unwrap();
    let options_id = INST_STORE.add(StatsigOptions::new()).unwrap();

    assert!(INST_STORE.get::<StatsigUser>(&user_id).is_some());
    assert!(INST_STORE.get::<Statsig>(&statsig_id).is_some());
    assert!(INST_STORE.get::<StatsigOptions>(&options_id).is_some());

    INST_STORE.remove(&user_id);
    INST_STORE.remove(&statsig_id);
    INST_STORE.remove(&options_id);

    assert!(INST_STORE.get::<StatsigUser>(&user_id).is_none());
    assert!(INST_STORE.get::<Statsig>(&statsig_id).is_none());
    assert!(INST_STORE.get::<StatsigOptions>(&options_id).is_none());
}
