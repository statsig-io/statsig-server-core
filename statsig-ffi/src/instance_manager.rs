use lazy_static::lazy_static;
use statsig::{log_d, log_e, Statsig, StatsigOptions, StatsigUser};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, RwLock};

lazy_static! {
    pub static ref STATSIG_INSTANCES: InstanceManager<Statsig> = InstanceManager::new();
    pub static ref OPTIONS_INSTANCES: InstanceManager<StatsigOptions> = InstanceManager::new();
    pub static ref USER_INSTANCES: InstanceManager<StatsigUser> = InstanceManager::new();
}

const MAX_ID_COUNTER: i64 = 10_000;

#[derive(Eq, PartialEq)]
enum InstanceType {
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
    fn to_prefix_value(self) -> i64 {
        (self as i64) << 32
    }

    fn from_id(id: i64) -> Option<InstanceType> {
        match id >> 32 {
            1 => Some(InstanceType::Statsig),
            2 => Some(InstanceType::StatsigOptions),
            3 => Some(InstanceType::StatsigUser),
            _ => None,
        }
    }
}

trait IsInstanceType {
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

pub struct InstanceManager<T: IsInstanceType> {
    instances: RwLock<HashMap<i64, Arc<T>>>,
    id_counter: AtomicI64,
}

impl<T: IsInstanceType> InstanceManager<T> {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            id_counter: AtomicI64::new(1),
        }
    }

    pub fn add(&self, inst: T) -> i64 {
        let id_prefix = T::get_instance_type().to_prefix_value();

        let next_id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        if next_id > MAX_ID_COUNTER {
            log_e!("Too many {} references created. Max ID limit reached.", T::get_instance_type());
            return -1;
        }

        let id = id_prefix | next_id;

        let mut instances = self.instances.write().unwrap();
        instances.insert(id, Arc::new(inst));
        log_d!("Added {} {}", T::get_instance_type(), id);
        id
    }

    pub fn get(&self, id: i64) -> Option<Arc<T>> {
        match InstanceType::from_id(id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                self.instances.read().unwrap().get(&id).cloned()
            }
            _ => {
                log_e!("Invalid ID {} for this {} type", id, T::get_instance_type());
                None
            },
        }

    }

    pub fn release(&self, id: i64) {
        match InstanceType::from_id(id) {
            Some(prefix) if prefix == T::get_instance_type() => {
                let mut instances = self.instances.write().unwrap();
                instances.remove(&id);
                log_d!("Released {} {}", T::get_instance_type(), id);
            }
            _ => {
                log_e!("Invalid ID {} for this {} type", id, T::get_instance_type());
            },
        }
    }
}


