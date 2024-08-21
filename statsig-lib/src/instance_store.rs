use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};

use crate::{log_d, log_e, log_w, Statsig, StatsigOptions, StatsigUser};

#[macro_export]
macro_rules! get_instance_or_noop {
    ($instances:ident, $ref:ident) => {
        match $instances.get($ref) {
            Some(instance) => instance,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return {
    ($instances:ident, $ref:ident, $ret_value:ident) => {
        match $instances.get($ref) {
            Some(instance) => instance,
            None => return $ret_value,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_else {
    ($instances:ident, $ref:ident, $else:expr) => {
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

const MAX_STORED_INSTANCES: usize = 10_000;
const MAX_ID_VALUE: i32 = 0x03FF_FFFF; // Max value for 26 bits

#[derive(Eq, PartialEq)]
pub enum InstanceType {
    Statsig = 1,
    StatsigOptions = 2,
    StatsigUser = 3,
    // Add more types as needed, up to 64
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
    fn to_prefix_value(self) -> i32 {
        (self as i32) << 26 // Shift left by 26 bits to reserve the top 6 bits for the instance type
    }

    fn from_id(id: i32) -> Option<InstanceType> {
        match (id >> 26) & 0x3F {
            // Extract the top 6 bits for the instance type
            1 => Some(InstanceType::Statsig),
            2 => Some(InstanceType::StatsigOptions),
            3 => Some(InstanceType::StatsigUser),
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

pub struct IdGenerator {
    counter: AtomicI32,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            counter: AtomicI32::new(0),
        }
    }

    pub fn next_id(&self, prefix: i32) -> i32 {
        loop {
            let current = self.counter.fetch_add(1, Ordering::Relaxed);

            // Wrap around if we exceed the max value for 26 bits
            let id = if current > MAX_ID_VALUE {
                log_w!("Counter Reset");
                self.counter.store(0, Ordering::Relaxed);
                prefix // Start fresh with the prefix
            } else {
                prefix | current
            };

            // Ensure that we have a valid non-negative ID
            if id >= 0 {
                return id;
            }
        }
    }
}

pub struct InstanceStore<T: IsInstanceType> {
    instances: RwLock<HashMap<i32, Arc<T>>>,
    id_generator: IdGenerator,
}

impl<T: IsInstanceType> InstanceStore<T> {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            id_generator: IdGenerator::new(),
        }
    }

    pub fn add(&self, inst: T) -> i32 {
        let mut instances = self.instances.write().unwrap();

        if instances.len() >= MAX_STORED_INSTANCES {
            let err_msg = format!(
                "Too many {} references created. Max ID limit reached.",
                T::get_instance_type()
            );
            log_e!("{}", err_msg);
            return -1;
        }

        let id_prefix = T::get_instance_type().to_prefix_value();
        let mut retries = 0;

        loop {
            let id = self.id_generator.next_id(id_prefix);

            // Check for collisions
            if !instances.contains_key(&id) {
                instances.insert(id, Arc::new(inst));
                log_d!("Added {} {}", T::get_instance_type(), id);
                return id;
            }

            retries += 1;
            if retries > 10 {
                let err_msg = format!(
                    "Failed to generate a unique ID for {} after multiple attempts.",
                    T::get_instance_type()
                );
                log_e!("{}", err_msg);
                return -1;
            }

            log_w!("Collision, retrying...");
        }
    }

    pub fn optional_get(&self, id: Option<i32>) -> Option<Arc<T>> {
        match id {
            Some(id) => self.get(id),
            None => None,
        }
    }

    pub fn get(&self, id: i32) -> Option<Arc<T>> {
        match InstanceType::from_id(id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                self.instances.read().unwrap().get(&id).cloned()
            }
            _ => {
                log_e!("Invalid ID {} for this {} type", id, T::get_instance_type());
                None
            }
        }
    }

    pub fn release(&self, id: i32) {
        match InstanceType::from_id(id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                let mut instances = self.instances.write().unwrap();
                instances.remove(&id);
                log_d!("Released {} {}", T::get_instance_type(), id);
            }
            _ => {
                log_e!("Invalid ID {} for this {} type", id, T::get_instance_type());
            }
        }
    }
}
