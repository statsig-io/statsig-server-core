use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::{
    log_d, log_e, log_w, Statsig, StatsigHttpEventLoggingAdapter, StatsigHttpSpecsAdapter,
    StatsigLocalFileEventLoggingAdapter, StatsigLocalFileSpecsAdapter, StatsigOptions, StatsigUser,
};

const TAG: &str = stringify!(InstanceStore);

#[macro_export]
macro_rules! get_instance_or_noop {
    ($type:ty, $ref:expr) => {
        match INST_STORE.get::<$type>($ref) {
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
        match INST_STORE.get::<$type>($ref) {
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
        match INST_STORE.get::<$type>($ref) {
            Some(instance) => instance,
            None => $else,
        }
    };
}

macro_rules! impl_boxable_instance {
    ($type:ty, $variant:ident, $prefix:expr) => {
        impl BoxableInstance for $type {
            fn from_box(boxed: &BoxedInstance) -> Option<Arc<Self>> {
                if let BoxedInstance::$variant(inner) = boxed {
                    Some(inner.clone())
                } else {
                    log_e!(TAG, "Invalid box type");
                    None
                }
            }

            fn into_box(self) -> BoxedInstance {
                BoxedInstance::$variant(Arc::new(self))
            }

            fn get_display_value_static() -> String {
                stringify!($type).to_string()
            }

            fn get_display_value(&self) -> String {
                stringify!($type).to_string()
            }

            fn get_prefix_value(&self) -> String {
                $prefix.to_string()
            }
        }
    };
}

macro_rules! impl_all_instances {
    ($(($type:ty, $variant:ident) => $prefix:expr),* $(,)?) => {
        $(
            impl_boxable_instance!($type, $variant, $prefix);
        )*
    }
}

lazy_static! {
    pub static ref INST_STORE: InstanceStore = InstanceStore::new();
}

const MAX_STORED_INSTANCES: usize = 400_000;

pub enum BoxedInstance {
    Statsig(Arc<Statsig>),
    StatsigOptions(Arc<StatsigOptions>),
    StatsigUser(Arc<StatsigUser>),
    StatsigHttpSpecsAdapter(Arc<StatsigHttpSpecsAdapter>),
    StatsigLocalFileSpecsAdapter(Arc<StatsigLocalFileSpecsAdapter>),
    StatsigHttpEventLoggingAdapter(Arc<StatsigHttpEventLoggingAdapter>),
    StatsigLocalFileEventLoggingAdapter(Arc<StatsigLocalFileEventLoggingAdapter>),
}

pub trait BoxableInstance {
    fn from_box(boxed: &BoxedInstance) -> Option<Arc<Self>>;
    fn into_box(self) -> BoxedInstance;
    fn get_display_value_static() -> String;
    fn get_display_value(&self) -> String;
    fn get_prefix_value(&self) -> String;
}

impl_all_instances! {
    (Statsig, Statsig) => "stsg",
    (StatsigOptions, StatsigOptions) => "opts",
    (StatsigUser, StatsigUser) => "usr",
    (StatsigHttpSpecsAdapter, StatsigHttpSpecsAdapter) => "spc_http",
    (StatsigLocalFileSpecsAdapter, StatsigLocalFileSpecsAdapter) => "spc_file",
    (StatsigHttpEventLoggingAdapter, StatsigHttpEventLoggingAdapter) => "evt_http",
    (StatsigLocalFileEventLoggingAdapter, StatsigLocalFileEventLoggingAdapter) => "evt_file",
}

pub struct InstanceStore {
    instances: RwLock<HashMap<String, BoxedInstance>>,
}

impl Default for InstanceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, boxable: impl BoxableInstance) -> Option<String> {
        let mut instances = self.instances.write().ok()?;

        if instances.len() >= MAX_STORED_INSTANCES {
            log_e!(
                TAG,
                "Too many {} references created. Max ID limit reached.",
                boxable.get_display_value()
            );
            return None;
        }

        let id_prefix = boxable.get_prefix_value();
        let mut retries = 0;

        loop {
            let id = format!("{}_{}", id_prefix, Uuid::new_v4());

            // Check for collisions
            if !instances.contains_key(&id) {
                log_d!(TAG, "Added {} {}", boxable.get_display_value(), &id);
                instances.insert(id.clone(), boxable.into_box());
                return Some(id);
            }

            retries += 1;
            if retries > 10 {
                let err_msg = format!(
                    "Failed to generate a unique ID for {} after multiple attempts.",
                    boxable.get_display_value()
                );
                log_e!(TAG, "{}", err_msg);
                return None;
            }

            log_w!(TAG, "Collision, retrying...");
        }
    }

    pub fn get<T: BoxableInstance>(&self, id: &str) -> Option<Arc<T>> {
        let instances = if let Ok(instances) = self.instances.read() {
            instances
        } else {
            log_e!(TAG, "Instance store is poisoned");
            return None;
        };

        let found = if let Some(inst) = instances.get(id) {
            inst
        } else {
            log_d!(
                TAG,
                "{} instance not found for ID {}",
                T::get_display_value_static(),
                id
            );
            return None;
        };

        if let Some(inst) = T::from_box(found) {
            Some(inst)
        } else {
            log_e!(
                TAG,
                "Invalid box type for {}",
                T::get_display_value_static()
            );
            None
        }
    }

    pub fn get_with_optional_id<T: BoxableInstance>(&self, id: Option<&String>) -> Option<Arc<T>> {
        match id {
            Some(id) => self.get(id),
            None => None,
        }
    }

    pub fn remove(&self, id: &str) -> Option<BoxedInstance> {
        self.instances.write().ok()?.remove(id)
    }

    pub fn remove_all(&self) {
        if let Ok(mut instances) = self.instances.write() {
            instances.clear();
        }
    }
}
