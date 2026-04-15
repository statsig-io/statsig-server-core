use crate::{global_configs::GlobalConfigs, hashing::HashUtil, DynamicValue, StatsigOptions};
use chrono::Utc;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

const SEC_EXPO_AS_PRIMARY_FLAG_PREFIX: &str = "sec_expo_as_primary:";
const SEC_EXPO_NUMBER_SDK_CONFIG: &str = "sec_expo_number";
const SEC_EXPO_BUCKET_SIZE: u64 = 1000;
const SEC_EXPO_NUMBER_REFRESH_INTERVAL_MS: u64 = 60_000;
const NO_SEC_EXPO_NUMBER: u64 = u64::MAX;

pub(super) struct SecExpoAsPrimaryExperiment {
    // The rollout bucket derived from StatsigOptions.experimental_flags.
    provided_exp_number: Option<u64>,
    global_configs: Arc<GlobalConfigs>,
    cached_sec_expo_number: AtomicU64,
    last_sec_expo_number_refresh_time: AtomicU64,
}

impl SecExpoAsPrimaryExperiment {
    pub(super) fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        let experiment_number = options
            .experimental_flags
            .as_ref()
            .and_then(|flags| {
                flags
                    .iter()
                    .filter_map(|flag| flag.strip_prefix(SEC_EXPO_AS_PRIMARY_FLAG_PREFIX))
                    .filter(|identifier| !identifier.is_empty())
                    .min()
            })
            .and_then(Self::get_experiment_number);

        Self {
            provided_exp_number: experiment_number,
            global_configs: GlobalConfigs::get_instance(sdk_key),
            cached_sec_expo_number: AtomicU64::new(NO_SEC_EXPO_NUMBER),
            last_sec_expo_number_refresh_time: AtomicU64::new(0),
        }
    }

    pub(super) fn should_log_as_primary(&self) -> bool {
        let Some(experiment_number) = self.provided_exp_number else {
            return false;
        };

        self.get_cached_sec_expo_number()
            .is_some_and(|sdk_config_number| experiment_number < sdk_config_number)
    }

    fn get_cached_sec_expo_number(&self) -> Option<u64> {
        let now = Utc::now().timestamp_millis() as u64;
        let last_refresh = self
            .last_sec_expo_number_refresh_time
            .load(Ordering::Relaxed);

        if self.should_refresh_sec_expo_number(now, last_refresh)
            && self
                .last_sec_expo_number_refresh_time
                .compare_exchange(last_refresh, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let sec_expo_number = self.global_configs.get_sec_expo_number();
            self.cached_sec_expo_number.store(
                sec_expo_number.unwrap_or(NO_SEC_EXPO_NUMBER),
                Ordering::Relaxed,
            );
            return sec_expo_number;
        }

        match self.cached_sec_expo_number.load(Ordering::Relaxed) {
            NO_SEC_EXPO_NUMBER => None,
            value => Some(value),
        }
    }

    fn should_refresh_sec_expo_number(&self, now: u64, last_refresh: u64) -> bool {
        last_refresh == 0 || now.saturating_sub(last_refresh) >= SEC_EXPO_NUMBER_REFRESH_INTERVAL_MS
    }

    fn get_experiment_number(identifier: &str) -> Option<u64> {
        let identifier = identifier.to_string();
        HashUtil::new()
            .evaluation_hash(&identifier)
            .map(|hash| hash % SEC_EXPO_BUCKET_SIZE)
    }
}

impl GlobalConfigs {
    fn get_sec_expo_number(&self) -> Option<u64> {
        fn parse_sec_expo_number(value: Option<&DynamicValue>) -> Option<u64> {
            let number = value?.float_value?;
            if !number.is_finite() {
                return None;
            }

            Some(number.clamp(0.0, SEC_EXPO_BUCKET_SIZE as f64) as u64)
        }

        self.use_sdk_config_value(SEC_EXPO_NUMBER_SDK_CONFIG, parse_sec_expo_number)
    }
}
