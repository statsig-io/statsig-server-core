use ahash::AHashMap;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockWriteGuard};
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::hashing::hash_one;
use crate::log_e;

type AnyInstance = Arc<dyn Any + Send + Sync>;

lazy_static! {
    static ref REGISTRY: RwLock<AHashMap<u64, AnyInstance>> = RwLock::new(AHashMap::default());
}

const TAG: &str = "InstanceRegistry";

pub struct InstanceRegistry;

impl InstanceRegistry {
    pub fn register_arc<T: Send + Sync + 'static>(instance: Arc<T>) -> Option<u64> {
        let full_type_name = std::any::type_name::<T>();
        let short_type_name = full_type_name.split("::").last().unwrap_or(full_type_name);
        let id_tuple = (short_type_name, Uuid::new_v4());
        let id_hash = hash_one(id_tuple);

        let mut registry = Self::get_write_lock()?;
        registry.insert(id_hash, instance);

        Some(id_hash)
    }

    pub fn register<T: Send + Sync + 'static>(instance: T) -> Option<u64> {
        Self::register_arc(Arc::new(instance))
    }

    pub fn get_with_optional_id<T: Send + Sync + 'static>(id: Option<&u64>) -> Option<Arc<T>> {
        id.and_then(|id_hash| Self::get::<T>(id_hash))
    }

    pub fn get<T: Send + Sync + 'static>(id: &u64) -> Option<Arc<T>> {
        let registry = match REGISTRY.try_read_for(Duration::from_secs(1)) {
            Some(guard) => guard,
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock REGISTRY");
                return None;
            }
        };

        registry
            .get(id)
            .and_then(|any_arc| match any_arc.clone().downcast::<T>() {
                Ok(t) => Some(t),
                Err(_) => {
                    log_e!(
                        TAG,
                        "Failed to downcast instance with ref '{}' to generic type",
                        id
                    );
                    None
                }
            })
    }

    pub fn get_raw(id: &u64) -> Option<Arc<dyn Any + Send + Sync>> {
        let registry = match REGISTRY.try_read_for(Duration::from_secs(1)) {
            Some(guard) => guard,
            None => {
                log_e!(TAG, "Failed to acquire read lock: Failed to lock REGISTRY");
                return None;
            }
        };

        registry.get(id).cloned()
    }

    pub fn remove(id: &u64) {
        let mut registry = match Self::get_write_lock() {
            Some(registry) => registry,
            None => return,
        };
        registry.remove(id);
    }

    pub fn remove_all() {
        let mut registry = match Self::get_write_lock() {
            Some(registry) => registry,
            None => return,
        };
        registry.clear();
    }

    fn get_write_lock() -> Option<RwLockWriteGuard<'static, AHashMap<u64, AnyInstance>>> {
        match REGISTRY.try_write_for(Duration::from_secs(1)) {
            Some(registry) => Some(registry),
            None => {
                log_e!(TAG, "Failed to acquire write lock: Failed to lock REGISTRY");
                None
            }
        }
    }
}

#[macro_export]
macro_rules! get_instance_or_noop {
    ($type:ty, $ref:expr) => {
        match statsig_rust::InstanceRegistry::get::<$type>($ref) {
            Some(instance) => instance,
            None => {
                $crate::log_w!(TAG, "{} Reference not found {}", stringify!($type), $ref);
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return {
    ($type:ty, $ref:expr, $return_val:expr) => {
        match statsig_rust::InstanceRegistry::get::<$type>($ref) {
            Some(instance) => instance,
            None => {
                $crate::log_w!(TAG, "{} Reference not found {}", stringify!($type), $ref);
                return $return_val;
            }
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_else {
    ($type:ty, $ref:expr, $else:expr) => {
        match statsig_rust::InstanceRegistry::get::<$type>($ref) {
            Some(instance) => instance,
            None => $else,
        }
    };
}
