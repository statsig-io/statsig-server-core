use super::event_queue::queued_event::{EnqueueOperation, QueuedExposure};
use crate::{
    global_configs::GlobalConfigs, hashing::HashUtil, write_lock_or_noop, write_lock_or_return,
    DynamicValue,
};
use chrono::Utc;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

const TAG: &str = "ExposureSampling";
const SAMPLING_TTL_MS: u64 = 60_000;
const SAMPLING_MAX_KEYS: usize = 100_000;

#[derive(Debug)]
pub enum EvtSamplingMode {
    On,
    Shadow,
}

#[derive(Debug)]
pub enum EvtSamplingDecision {
    Deduped,
    NotSampled,
    ForceSampled,
    Sampled(Option<u64>, EvtSamplingMode, bool),
}

impl EvtSamplingDecision {
    pub fn should_log(&self) -> bool {
        match self {
            EvtSamplingDecision::Deduped | EvtSamplingDecision::NotSampled => false,
            EvtSamplingDecision::ForceSampled | EvtSamplingDecision::Sampled(_, _, _) => true,
        }
    }
}

pub struct ExposureSampling {
    spec_sampling_set: RwLock<HashSet<String>>,
    last_spec_sampling_reset: AtomicU64,

    exposure_dedupe_set: RwLock<HashSet<String>>,
    last_exposure_dedupe_reset: AtomicU64,

    global_configs: Arc<GlobalConfigs>,
    hasher: HashUtil,
}

impl ExposureSampling {
    pub fn new(sdk_key: &str) -> Self {
        Self {
            spec_sampling_set: RwLock::from(HashSet::new()),
            last_spec_sampling_reset: AtomicU64::from(0),

            exposure_dedupe_set: RwLock::from(HashSet::new()),
            last_exposure_dedupe_reset: AtomicU64::from(0),

            global_configs: GlobalConfigs::get_instance(sdk_key),
            hasher: HashUtil::new(),
        }
    }

    pub fn get_sampling_decision(&self, payload: &impl EnqueueOperation) -> EvtSamplingDecision {
        let exposure = match payload.as_exposure() {
            Some(exposure) => exposure,
            None => return EvtSamplingDecision::ForceSampled,
        };

        let expo_sampling_key = exposure.create_exposure_sampling_key();
        if self.should_dedupe_exposure(&expo_sampling_key) {
            return EvtSamplingDecision::Deduped;
        }

        let sampling_mode = match self.global_configs.get_sampling_mode() {
            Some(sampling_mode) => sampling_mode,
            None => return EvtSamplingDecision::ForceSampled,
        };

        if self.should_sample_based_on_evaluation(exposure) {
            return EvtSamplingDecision::ForceSampled;
        }

        if self.should_sample_first_time_exposure(exposure) {
            return EvtSamplingDecision::ForceSampled;
        }

        let sampling_rate = self
            .get_special_case_sampling_rate(exposure)
            .or_else(|| exposure.get_sampling_rate());

        let is_sampled = self.is_sampled(&expo_sampling_key, sampling_rate);

        match sampling_mode {
            EvtSamplingMode::On if is_sampled => {
                EvtSamplingDecision::Sampled(sampling_rate, EvtSamplingMode::On, true)
            }
            EvtSamplingMode::Shadow => {
                EvtSamplingDecision::Sampled(sampling_rate, EvtSamplingMode::Shadow, is_sampled)
            }
            _ => EvtSamplingDecision::NotSampled,
        }
    }

    pub fn try_reset_all_sampling(&self) {
        self.try_reset_exposure_dedupe_set();
        self.try_reset_spec_sampling_set();
    }

    fn should_dedupe_exposure(&self, full_sampling_key: &String) -> bool {
        let mut dedupe_set = write_lock_or_return!(TAG, self.exposure_dedupe_set, false);
        if dedupe_set.contains(full_sampling_key) {
            return true;
        }

        dedupe_set.insert(full_sampling_key.to_owned());

        false
    }

    fn should_sample_based_on_evaluation<'a>(&self, exposure: &'a impl QueuedExposure<'a>) -> bool {
        let sampling_info = match exposure.get_sampling_info_ref() {
            Some(sampling_info) => sampling_info,
            None => return false,
        };

        if sampling_info.forward_all_exposures == Some(true) {
            return true;
        }

        if sampling_info.has_seen_analytical_gates == Some(true) {
            return true;
        }

        false
    }

    fn should_sample_first_time_exposure<'a>(&self, exposure: &impl QueuedExposure<'a>) -> bool {
        let sampling_key = exposure.create_spec_sampling_key();
        if self.sample_key_exists(&sampling_key) {
            return false;
        }

        if let Ok(mut sampling_map) = self.spec_sampling_set.try_write() {
            sampling_map.insert(sampling_key);
        }

        true
    }

    fn is_sampled(&self, sampling_key: &str, sampling_rate: Option<u64>) -> bool {
        let sampling_rate = match sampling_rate {
            Some(rate) => rate,
            None => return true, // without a sampling rate, we should sample
        };

        let hash = self.hasher.sha256_to_u64(sampling_key);
        hash % sampling_rate == 0
    }

    fn try_reset_spec_sampling_set(&self) {
        let now = Utc::now().timestamp_millis() as u64;
        let last_sampling_reset = self.last_spec_sampling_reset.load(Ordering::Relaxed);
        let mut sampling_map = write_lock_or_noop!(TAG, self.spec_sampling_set);

        let has_expired = now - last_sampling_reset > SAMPLING_TTL_MS;
        let is_full = sampling_map.len() > SAMPLING_MAX_KEYS;

        if has_expired || is_full {
            sampling_map.clear();
            self.last_spec_sampling_reset.store(now, Ordering::Relaxed);
        }
    }

    fn try_reset_exposure_dedupe_set(&self) {
        let now = Utc::now().timestamp_millis() as u64;
        let last_dedupe_reset = self.last_exposure_dedupe_reset.load(Ordering::Relaxed);
        let mut dedupe_map = self.exposure_dedupe_set.write().unwrap();

        let has_expired = now - last_dedupe_reset > SAMPLING_TTL_MS;
        let is_full = dedupe_map.len() > SAMPLING_MAX_KEYS;

        if has_expired || is_full {
            dedupe_map.clear();
            self.last_exposure_dedupe_reset
                .store(now, Ordering::Relaxed);
        }
    }

    fn sample_key_exists(&self, key: &str) -> bool {
        match self.spec_sampling_set.try_read() {
            Ok(map) => map.contains(key),
            _ => false,
        }
    }

    fn get_special_case_sampling_rate<'a>(
        &self,
        exposure: &'a impl QueuedExposure<'a>,
    ) -> Option<u64> {
        let rule_id = exposure.get_rule_id_ref();
        match rule_id {
            "default" | "disabled" | "" => self.global_configs.get_special_case_sampling_rate(),
            _ => None,
        }
    }
}

impl GlobalConfigs {
    fn get_sampling_mode(&self) -> Option<EvtSamplingMode> {
        fn parse_sampling_mode(sampling_mode: Option<&DynamicValue>) -> Option<EvtSamplingMode> {
            let v = sampling_mode?.string_value.as_ref()?.value.as_str();

            match v {
                "on" => Some(EvtSamplingMode::On),
                "shadow" => Some(EvtSamplingMode::Shadow),
                _ => None,
            }
        }

        self.use_sdk_config_value("sampling_mode", parse_sampling_mode)
    }

    fn get_special_case_sampling_rate(&self) -> Option<u64> {
        fn parse_special_case_sampling_rate(value: Option<&DynamicValue>) -> Option<u64> {
            match value {
                Some(value) => value.float_value.map(|rate| rate as u64),
                None => None,
            }
        }

        self.use_sdk_config_value(
            "special_case_sampling_rate",
            parse_special_case_sampling_rate,
        )
    }
}
