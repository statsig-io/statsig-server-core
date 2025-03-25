use lazy_static::lazy_static;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use uuid::Uuid;

use crate::log_e;

type AnyInstance = Arc<dyn Any + Send + Sync>;

lazy_static! {
    static ref REGISTRY: RwLock<HashMap<String, AnyInstance>> = RwLock::new(HashMap::new());
}

const TAG: &str = "InstanceRegistry";

pub struct InstanceRegistry;

impl InstanceRegistry {
    pub fn register_arc<T: Send + Sync + 'static>(instance: Arc<T>) -> Option<String> {
        let full_type_name = std::any::type_name::<T>();
        let short_type_name = full_type_name.split("::").last().unwrap_or(full_type_name);
        let id = format!("{}_{}", short_type_name, Uuid::new_v4());

        let mut registry = Self::get_write_lock()?;

        registry.insert(id.clone(), instance);

        Some(id)
    }

    pub fn register<T: Send + Sync + 'static>(instance: T) -> Option<String> {
        Self::register_arc(Arc::new(instance))
    }

    pub fn get_with_optional_id<T: Send + Sync + 'static>(id: Option<&String>) -> Option<Arc<T>> {
        id.and_then(|id_str| Self::get::<T>(id_str))
    }

    pub fn get<T: Send + Sync + 'static>(id: &str) -> Option<Arc<T>> {
        let registry = match REGISTRY.read() {
            Ok(guard) => guard,
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock: {}", e);
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

    pub fn remove(id: &str) {
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

    fn get_write_lock() -> Option<RwLockWriteGuard<'static, HashMap<String, AnyInstance>>> {
        match REGISTRY.write() {
            Ok(registry) => Some(registry),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock: {}", e);
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
