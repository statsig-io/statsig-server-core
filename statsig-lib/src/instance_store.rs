use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::{log_d, log_e, log_w, Statsig, StatsigOptions, StatsigUser};

#[macro_export]
macro_rules! get_instance_or_noop {
    ($instances:ident, $ref:expr) => {
        match $instances.get($ref) {
            Some(instance) => instance,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return {
    ($instances:ident, $ref:expr, $ret_value:ident) => {
        match $instances.get($ref) {
            Some(instance) => instance,
            None => return $ret_value,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_else {
    ($instances:ident, $ref:expr, $else:expr) => {
        match $instances.get($ref) {
            Some(instance) => instance,
            None => $else,
        }
    };
}

lazy_static! {
    pub static ref STATSIG_INSTANCES: InstanceStore<Statsig> = InstanceStore::new();
    pub static ref OPTIONS_INSTANCES: InstanceStore<StatsigOptions> = InstanceStore::new();
    pub static ref USER_INSTANCES: InstanceStore<StatsigUser> = InstanceStore::new();
}

const MAX_STORED_INSTANCES: usize = 100_000;

#[derive(Eq, PartialEq)]
pub enum InstanceType {
    Statsig = 1,
    StatsigOptions = 2,
    StatsigUser = 3,
}

impl Display for InstanceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let type_name = match self {
            InstanceType::Statsig => "Statsig",
            InstanceType::StatsigOptions => "StatsigOptions",
            InstanceType::StatsigUser => "StatsigUser",
        };
        write!(f, "{}", type_name)
    }
}

impl InstanceType {
    fn to_prefix_value(&self) -> String {
        match self {
            InstanceType::Statsig => "stsg".to_string(),
            InstanceType::StatsigOptions => "opts".to_string(),
            InstanceType::StatsigUser => "usr".to_string(),
        }
    }

    fn from_id(id: &str) -> Option<InstanceType> {
        let prefix = id.split('_').next();
        match prefix {
            Some("stsg") => Some(InstanceType::Statsig),
            Some("opts") => Some(InstanceType::StatsigOptions),
            Some("usr") => Some(InstanceType::StatsigUser),
            _ => None,
        }
    }
}

pub trait IsInstanceType {
    fn get_instance_type() -> InstanceType;
}

impl IsInstanceType for Statsig {
    fn get_instance_type() -> InstanceType {
        InstanceType::Statsig
    }
}

impl IsInstanceType for StatsigOptions {
    fn get_instance_type() -> InstanceType {
        InstanceType::StatsigOptions
    }
}

impl IsInstanceType for StatsigUser {
    fn get_instance_type() -> InstanceType {
        InstanceType::StatsigUser
    }
}

pub struct InstanceStore<T: IsInstanceType> {
    instances: RwLock<HashMap<String, Arc<T>>>,
}

impl<T: IsInstanceType> Default for InstanceStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IsInstanceType> InstanceStore<T> {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, inst: T) -> Option<String> {
        // todo: avoid unwrap
        let mut instances = self.instances.write().unwrap();

        if instances.len() >= MAX_STORED_INSTANCES {
            log_e!(
                "Too many {} references created. Max ID limit reached.",
                T::get_instance_type()
            );
            return None;
        }

        let id_prefix = T::get_instance_type().to_prefix_value();
        let mut retries = 0;

        loop {
            let id = format!("{}_{}", id_prefix, Uuid::new_v4());

            // Check for collisions
            if !instances.contains_key(&id) {
                log_d!("Added {} {}", T::get_instance_type(), &id);
                instances.insert(id.clone(), Arc::new(inst));
                return Some(id);
            }

            retries += 1;
            if retries > 10 {
                let err_msg = format!(
                    "Failed to generate a unique ID for {} after multiple attempts.",
                    T::get_instance_type()
                );
                log_e!("{}", err_msg);
                return None;
            }

            log_w!("Collision, retrying...");
        }
    }

    pub fn optional_get(&self, id: Option<&String>) -> Option<Arc<T>> {
        match id {
            Some(id) => self.get(id),
            None => None,
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<T>> {
        // todo: avoid unwrap
        match InstanceType::from_id(id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                self.instances.read().unwrap().get(id).cloned()
            }
            _ => {
                log_e!("Invalid ID {} for {} type", id, T::get_instance_type());
                None
            }
        }
    }

    pub fn release(&self, id: String) {
        match InstanceType::from_id(&id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                let mut instances = self.instances.write().unwrap();
                instances.remove(&id);
                log_d!("Released {} with ID {}", T::get_instance_type(), id);
            }
            _ => {
                log_e!("Invalid ID {} for {} type", id, T::get_instance_type());
            }
        }
    }

    pub fn release_all(&self) {
        let mut instances = self.instances.write().unwrap();
        instances.clear();
    }
}
