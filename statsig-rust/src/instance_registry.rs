use dashmap::DashMap;
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::Arc;
use uuid::Uuid;

use crate::hashing::hash_one;
use crate::log_e;

type AnyInstance = Arc<dyn Any + Send + Sync>;

// S2SDK-16 fix: the instance registry is on the SDK evaluation hot path. Every
// check_gate / get_experiment call resolves the Statsig and StatsigUser
// handles through it, and FFI bindings (e.g. Java/JNI) create + release a
// StatsigUser per request. The previous design was a single global
// `RwLock<AHashMap>`: at high QPS the per-request write locks (user
// create/release) blocked every concurrent eval read lock, serializing an
// otherwise many-core workload and starving CPU.
//
// DashMap is a sharded concurrent map (an array of independently locked
// buckets keyed by hash), so reads/writes to different ids touch different
// shards and no longer contend on one global lock. As a bonus it also removes
// the blanket 5s lock-timeout stalls the old macro-based locking produced
// under contention.
type Registry = DashMap<u64, AnyInstance, ahash::RandomState>;

lazy_static! {
    static ref REGISTRY: Registry = DashMap::with_hasher(ahash::RandomState::new());
}

const TAG: &str = "InstanceRegistry";

pub struct InstanceRegistry;

impl InstanceRegistry {
    pub fn register_arc<T: Send + Sync + 'static>(instance: Arc<T>) -> Option<u64> {
        let full_type_name = std::any::type_name::<T>();
        let short_type_name = full_type_name.split("::").last().unwrap_or(full_type_name);
        let id_tuple = (short_type_name, Uuid::new_v4());
        let id_hash = hash_one(id_tuple);

        // Insert only write-locks the single shard this id maps to.
        REGISTRY.insert(id_hash, instance);

        Some(id_hash)
    }

    pub fn register<T: Send + Sync + 'static>(instance: T) -> Option<u64> {
        Self::register_arc(Arc::new(instance))
    }

    pub fn get_with_optional_id<T: Send + Sync + 'static>(id: Option<&u64>) -> Option<Arc<T>> {
        id.and_then(|id_hash| Self::get::<T>(id_hash))
    }

    pub fn get<T: Send + Sync + 'static>(id: &u64) -> Option<Arc<T>> {
        // Clone the Arc out and release the shard guard immediately (the Ref
        // does not outlive this statement) so we never hold a DashMap guard
        // across other registry calls - that is what could otherwise deadlock
        // a sharded map.
        let any_arc = {
            let entry = REGISTRY.get(id)?;
            entry.value().clone()
        };

        match any_arc.downcast::<T>() {
            Ok(t) => Some(t),
            Err(_) => {
                log_e!(
                    TAG,
                    "Failed to downcast instance with ref '{}' to generic type",
                    id
                );
                None
            }
        }
    }

    pub fn get_raw(id: &u64) -> Option<Arc<dyn Any + Send + Sync>> {
        REGISTRY.get(id).map(|entry| entry.value().clone())
    }

    pub fn remove(id: &u64) {
        // Only write-locks the single shard this id maps to.
        REGISTRY.remove(id);
    }

    pub fn remove_all() {
        REGISTRY.clear();
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
