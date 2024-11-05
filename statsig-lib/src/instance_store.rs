use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::{
    log_d, log_e, log_w, Statsig, StatsigHttpSpecsAdapter, StatsigLocalFileSpecsAdapter,
    StatsigOptions, StatsigUser,
};

#[macro_export]
macro_rules! get_instance_or_noop {
    ($type:ty, $ref:expr) => {
        match INST_STORE.get::<$type>($ref) {
            Some(instance) => instance,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return {
    ($type:ty, $ref:expr, $return_val:expr) => {
        match INST_STORE.get::<$type>($ref) {
            Some(instance) => instance,
            None => return $return_val,
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
                    log_e!("Invalid box type");
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
}

pub trait BoxableInstance {
    fn from_box(boxed: &BoxedInstance) -> Option<Arc<Self>>;
    fn into_box(self) -> BoxedInstance;
    fn get_display_value_static() -> String;
    fn get_display_value(&self) -> String;
    fn get_prefix_value(&self) -> String;
}

impl_boxable_instance!(Statsig, Statsig, "stsg");
impl_boxable_instance!(StatsigOptions, StatsigOptions, "opts");
impl_boxable_instance!(StatsigUser, StatsigUser, "usr");
impl_boxable_instance!(
    StatsigHttpSpecsAdapter,
    StatsigHttpSpecsAdapter,
    "http_spcs"
);
impl_boxable_instance!(
    StatsigLocalFileSpecsAdapter,
    StatsigLocalFileSpecsAdapter,
    "file_spcs"
);

pub struct InstanceStore {
    instances: RwLock<HashMap<String, BoxedInstance>>,
}

impl InstanceStore {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub fn adda(&self, boxable: impl BoxableInstance) -> Option<String> {
        let id = boxable.get_prefix_value();
        self.instances
            .write()
            .unwrap()
            .insert(id.clone(), boxable.into_box());

        Some(id)
    }

    pub fn add(&self, boxable: impl BoxableInstance) -> Option<String> {
        let mut instances = self.instances.write().ok()?;

        if instances.len() >= MAX_STORED_INSTANCES {
            log_e!(
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
                log_d!("Added {} {}", boxable.get_display_value(), &id);
                instances.insert(id.clone(), boxable.into_box());
                return Some(id);
            }

            retries += 1;
            if retries > 10 {
                let err_msg = format!(
                    "Failed to generate a unique ID for {} after multiple attempts.",
                    boxable.get_display_value()
                );
                log_e!("{}", err_msg);
                return None;
            }

            log_w!("Collision, retrying...");
        }
    }

    pub fn get<T: BoxableInstance>(&self, id: &str) -> Option<Arc<T>> {
        let instances = match self.instances.read().ok() {
            Some(instances) => instances,
            None => {
                log_e!("Instance store is poisoned");
                return None;
            }
        };

        let found = match instances.get(id) {
            Some(inst) => inst,
            None => {
                log_d!(
                    "{} instance not found for ID {}",
                    T::get_display_value_static(),
                    id
                );
                return None;
            }
        };

        match T::from_box(found) {
            Some(inst) => Some(inst),
            None => {
                log_e!("Invalid box type for {}", T::get_display_value_static());
                None
            }
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
