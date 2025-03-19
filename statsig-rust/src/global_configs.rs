use crate::{log_d, log_e, DynamicValue};
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Weak},
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
        match GLOBAL_CONFIG_INSTANCES.read() {
            Ok(read_guard) => {
                if let Some(instance) = read_guard.get(sdk_key) {
                    if let Some(instance) = instance.upgrade() {
                        return instance.clone();
                    }
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to get read guard: {}", e);
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

        match GLOBAL_CONFIG_INSTANCES.write() {
            Ok(mut write_guard) => {
                write_guard.insert(sdk_key.into(), Arc::downgrade(&instance));
            }
            Err(e) => {
                log_e!(TAG, "Failed to get write guard: {}", e);
            }
        }

        instance
    }

    pub fn set_sdk_configs(&self, new_configs: HashMap<String, DynamicValue>) {
        match self.configs.write() {
            Ok(mut configs_guard) => {
                for (key, value) in new_configs {
                    configs_guard.sdk_configs.insert(key, value);
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to get write guard: {}", e);
            }
        }
    }

    pub fn set_diagnostics_sampling_rates(&self, new_sampling_rate: HashMap<String, f64>) {
        match self.configs.write() {
            Ok(mut configs_guard) => {
                for (key, rate) in new_sampling_rate {
                    let clamped_rate = rate.clamp(0.0, MAX_SAMPLING_RATE);
                    configs_guard
                        .diagnostics_sampling_rates
                        .insert(key, clamped_rate);
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to get write guard: {}", e);
            }
        }
    }

    pub fn get_sdk_config_value(&self, key: &str) -> Option<DynamicValue> {
        match self.configs.read() {
            Ok(configs_guard) => match configs_guard.sdk_configs.get(key) {
                Some(value) => Some(value.clone()),
                None => {
                    log_d!(TAG, "SDK Configs not found");
                    None
                }
            },
            Err(e) => {
                log_e!(TAG, "Failed to get read guard: {}", e);
                None
            }
        }
    }

    pub fn get_diagnostics_sampling_rate(&self) -> HashMap<String, f64> {
        match self.configs.read() {
            Ok(configs_guard) => configs_guard.diagnostics_sampling_rates.clone(),
            Err(e) => {
                log_e!(TAG, "Failed to get read guard: {}", e);
                HashMap::new()
            }
        }
    }
}
