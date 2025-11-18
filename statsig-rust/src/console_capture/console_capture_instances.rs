use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use crate::console_capture::console_log_line_levels::StatsigLogLineLevel;
use parking_lot::RwLock;

use crate::{
    log_e, observability::ops_stats::OpsStatsForInstance, user::StatsigUserLoggable, DynamicValue,
    StatsigOptions, OPS_STATS,
};

const TAG: &str = stringify!(ConsoleCaptureRegistry);

lazy_static! {
    pub static ref CONSOLE_CAPTURE_REGISTRY: ConsoleCaptureRegistry = ConsoleCaptureRegistry {
        instances_map: RwLock::new(HashMap::new())
    };
}

pub struct ConsoleCaptureInstance {
    enabled: bool,
    allowed_log_levels: Vec<StatsigLogLineLevel>,
    ops_stats_instance: Arc<OpsStatsForInstance>,
    console_capture_user: StatsigUserLoggable,
}

pub struct ConsoleCaptureRegistry {
    instances_map: RwLock<HashMap<String, Weak<ConsoleCaptureInstance>>>,
}

impl ConsoleCaptureInstance {
    pub fn new(
        sdk_key: &str,
        statsig_options: &StatsigOptions,
        environment: &Option<HashMap<String, DynamicValue>>,
    ) -> Self {
        let console_capture_options = statsig_options
            .console_capture_options
            .clone()
            .unwrap_or_default();
        let enabled = console_capture_options.enabled;
        let ops_stats_instance = OPS_STATS.get_for_instance(sdk_key);
        let allowed_log_levels = console_capture_options
            .log_levels
            .filter(|levels| !levels.is_empty())
            .unwrap_or(vec![StatsigLogLineLevel::Warn, StatsigLogLineLevel::Error]);

        let loggable_user = if let Some(console_capture_user) = console_capture_options.user {
            StatsigUserLoggable::new(
                &console_capture_user.data,
                environment.clone(),
                statsig_options.global_custom_fields.clone(),
            )
        } else {
            StatsigUserLoggable::default_console_capture_user(
                environment.clone(),
                statsig_options.global_custom_fields.clone(),
            )
        };

        Self {
            enabled,
            ops_stats_instance,
            allowed_log_levels,
            console_capture_user: loggable_user,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl ConsoleCaptureRegistry {
    pub fn get_for_instance(
        &self,
        sdk_key: &str,
        options: &StatsigOptions,
        environment: &Option<HashMap<String, DynamicValue>>,
    ) -> Arc<ConsoleCaptureInstance> {
        match self
            .instances_map
            .try_read_for(std::time::Duration::from_secs(5))
        {
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
                    "Failed to get read guard: Failed to lock instances_map"
                );
            }
        }

        let instance = Arc::new(ConsoleCaptureInstance::new(sdk_key, options, environment));
        match self
            .instances_map
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut write_guard) => {
                write_guard.insert(sdk_key.into(), Arc::downgrade(&instance));
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to get write guard: Failed to lock instances_map"
                );
            }
        }

        instance
    }

    /// Helper method to get a strong reference to an instance
    fn get_and_upgrade_instance_for_key(
        &self,
        sdk_key: &str,
    ) -> Option<Arc<ConsoleCaptureInstance>> {
        match self
            .instances_map
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(read_guard) => read_guard.get(sdk_key).cloned()?.upgrade(),
            None => None,
        }
    }

    pub fn is_enabled(&self, sdk_key: &str) -> bool {
        let instance = self.get_and_upgrade_instance_for_key(sdk_key);

        instance
            .map(|instance| instance.is_enabled())
            .unwrap_or(false)
    }

    pub fn enqueue_console_capture_event(
        &self,
        sdk_key: &str,
        level: String,
        payload: Vec<String>,
        timestamp: u64,
        stack_trace: Option<String>,
    ) {
        if !self.is_enabled(sdk_key) {
            return;
        }

        if is_internal_log(&payload) {
            return;
        }

        let Some(log_level) = StatsigLogLineLevel::from_string(&level) else {
            log_e!(TAG, "Failed to parse log level: {}", level);
            return;
        };

        let Some(instance) = self.get_and_upgrade_instance_for_key(sdk_key) else {
            log_e!(
                TAG,
                "Failed to get and upgrade instance for key: {}",
                sdk_key
            );
            return;
        };

        if !instance.allowed_log_levels.contains(&log_level) {
            log_e!(TAG, "Log level not allowed: {:?}", log_level);
            return;
        }

        let user = instance.console_capture_user.clone();

        instance.ops_stats_instance.enqueue_console_capture_event(
            level,
            payload,
            timestamp,
            user,
            stack_trace,
        );
    }
}

fn is_internal_log(payload: &[String]) -> bool {
    payload
        .iter()
        .any(|p| p.contains("Statsig::") || p.contains("[Statsig]"))
}
