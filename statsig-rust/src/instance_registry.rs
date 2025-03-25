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
    pub fn register<T: Send + Sync + 'static>(instance: Arc<T>) -> Option<String> {
        let full_type_name = std::any::type_name::<T>();
        let short_type_name = full_type_name.split("::").last().unwrap_or(full_type_name);
        let id = format!("{}_{}", short_type_name, Uuid::new_v4());

        let mut registry = match Self::get_write_lock() {
            Some(registry) => registry,
            None => return None,
        };

        registry.insert(id.clone(), instance);

        Some(id)
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
