use crate::{log_e, DynamicValue};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};

const TAG: &str = stringify!(GlobalConfigs);

pub const MAX_SAMPLING_RATE: f64 = 10000.0;

lazy_static! {
    static ref GLOBAL_CONFIG_INSTANCES: RwLock<HashMap<String, Weak<GlobalConfigs>>> =
        RwLock::new(HashMap::new());
}

struct Configs {
    sdk_configs: HashMap<String, DynamicValue>,
    diagnostics_sampling_rates: HashMap<String, f64>,
}

pub struct GlobalConfigs {
    configs: RwLock<Configs>,
}

impl GlobalConfigs {
    pub fn get_instance(sdk_key: &str) -> Arc<GlobalConfigs> {
        match GLOBAL_CONFIG_INSTANCES.try_read_for(Duration::from_secs(1)) {
            Some(read_guard) => {
                if let Some(instance) = read_guard.get(sdk_key) {
                    if let Some(instance) = instance.upgrade() {
                        return instance.clone();
                    }
                }
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to get read guard: Failed to lock GLOBAL_CONFIG_INSTANCES"
                );
            }
        }

        let instance = Arc::new(GlobalConfigs {
            configs: RwLock::new(Configs {
                sdk_configs: HashMap::new(),
                diagnostics_sampling_rates: HashMap::from([
                    ("initialize".to_string(), 10000.0),
                    ("config_sync".to_string(), 1000.0),
                    ("dcs".to_string(), 1000.0),
                    ("get_id_list".to_string(), 100.0), // default sampling rates
                ]),
            }),
        });

        match GLOBAL_CONFIG_INSTANCES.try_write_for(Duration::from_secs(1)) {
            Some(mut write_guard) => {
                write_guard.insert(sdk_key.into(), Arc::downgrade(&instance));
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to get write guard: Failed to lock GLOBAL_CONFIG_INSTANCES"
                );
            }
        }

        instance
    }

    pub fn set_sdk_configs(&self, new_configs: HashMap<String, DynamicValue>) {
        match self.configs.try_write_for(Duration::from_secs(1)) {
            Some(mut configs_guard) => {
                for (key, value) in new_configs {
                    configs_guard.sdk_configs.insert(key, value);
                }
            }
            None => {
                log_e!(TAG, "Failed to get write guard: Failed to lock configs");
            }
        }
    }

    pub fn set_diagnostics_sampling_rates(&self, new_sampling_rate: HashMap<String, f64>) {
        match self.configs.try_write_for(Duration::from_secs(1)) {
            Some(mut configs_guard) => {
                for (key, rate) in new_sampling_rate {
                    let clamped_rate = rate.clamp(0.0, MAX_SAMPLING_RATE);
                    configs_guard
                        .diagnostics_sampling_rates
                        .insert(key, clamped_rate);
                }
            }
            None => {
                log_e!(TAG, "Failed to get write guard: Failed to lock configs");
            }
        }
    }

    pub fn use_sdk_config_value<T>(
        &self,
        key: &str,
        f: impl FnOnce(Option<&DynamicValue>) -> T,
    ) -> T {
        match self.configs.try_read_for(Duration::from_secs(1)) {
            Some(configs_guard) => f(configs_guard.sdk_configs.get(key)),
            None => {
                log_e!(TAG, "Failed to get read guard: Failed to lock configs");
                f(None)
            }
        }
    }

    pub fn use_diagnostics_sampling_rate<T>(
        &self,
        key: &str,
        f: impl FnOnce(Option<&f64>) -> T,
    ) -> T {
        match self.configs.try_read_for(Duration::from_secs(1)) {
            Some(configs_guard) => f(configs_guard.diagnostics_sampling_rates.get(key)),
            None => {
                log_e!(TAG, "Failed to get read guard: Failed to lock configs");
                f(None)
            }
        }
    }
}
