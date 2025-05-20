use std::sync::{Arc, Mutex, MutexGuard};

use lazy_static::lazy_static;
use more_asserts::assert_gt;
use statsig_rust::instance_registry::InstanceRegistry;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn get_test_lock() -> MutexGuard<'static, ()> {
    let guard = TEST_MUTEX.lock().unwrap();

    InstanceRegistry::remove_all();

    guard
}

#[derive(Debug)]
pub struct MyBar {
    pub is_active: bool,
    pub data: String,
}

#[derive(Debug)]
pub struct MyFoo {
    pub name: String,
    pub bar: Arc<MyBar>,
}

#[test]
fn test_register_and_get() {
    let _lock = get_test_lock();

    let my_bar = MyBar {
        is_active: true,
        data: "bar".to_string(),
    };
    let id = InstanceRegistry::register(my_bar).unwrap();

    let retrieved = InstanceRegistry::get::<MyBar>(&id);
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().is_active);
}

#[test]
fn test_remove() {
    let _lock = get_test_lock();

    let my_bar = MyBar {
        is_active: true,
        data: "bar".to_string(),
    };
    let id = InstanceRegistry::register(my_bar).unwrap();

    InstanceRegistry::remove(&id);
    let retrieved = InstanceRegistry::get::<MyBar>(&id);
    assert!(retrieved.is_none());
}

#[test]
fn test_remove_all() {
    let _lock = get_test_lock();

    let my_bar = MyBar {
        is_active: true,
        data: "bar".to_string(),
    };
    let id = InstanceRegistry::register(my_bar).unwrap();

    InstanceRegistry::remove_all();
    let retrieved = InstanceRegistry::get::<MyBar>(&id);
    assert!(retrieved.is_none());
}

#[test]
fn test_register_and_get_nested() {
    let _lock = get_test_lock();

    let my_bar = MyBar {
        is_active: true,
        data: "bar".to_string(),
    };
    let my_foo = MyFoo {
        name: "foo".to_string(),
        bar: Arc::new(my_bar),
    };
    let id = InstanceRegistry::register(my_foo).unwrap();

    let retrieved = InstanceRegistry::get::<MyFoo>(&id).unwrap();
    assert!(retrieved.bar.is_active);
    assert_eq!(retrieved.bar.data, "bar");
    assert_eq!(retrieved.name, "foo");
}

#[test]
fn test_getting_wrong_type() {
    let _lock = get_test_lock();

    let my_bar = MyBar {
        is_active: true,
        data: "bar".to_string(),
    };
    let id = InstanceRegistry::register(my_bar).unwrap();

    let retrieved = InstanceRegistry::get::<MyFoo>(&id);
    assert!(retrieved.is_none());
}

#[test]
fn test_returns_valid_ids() {
    let _lock = get_test_lock();

    let my_bar = Arc::new(MyBar {
        is_active: true,
        data: "bar".to_string(),
    });
    let id = InstanceRegistry::register_arc(my_bar.clone()).unwrap();
    assert_gt!(id, 0);

    let my_foo = MyFoo {
        name: "foo".to_string(),
        bar: my_bar.clone(),
    };
    let id = InstanceRegistry::register(my_foo).unwrap();
    assert_gt!(id, 0);
}
