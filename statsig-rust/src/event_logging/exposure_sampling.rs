use super::event_queue::queued_event::{EnqueueOperation, QueuedExposure};
use crate::{
    evaluation::evaluation_types::{BaseEvaluation, ExtraExposureInfo},
    global_configs::GlobalConfigs,
    log_d, log_e,
    user::user_data::UserData,
    write_lock_or_noop, write_lock_or_return, DynamicValue,
};
use ahash::AHashSet;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
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

type SpecAndRuleHashTuple = (u64, u64);
pub struct ExposureSampling {
    spec_sampling_set: RwLock<AHashSet<SpecAndRuleHashTuple>>,
    last_spec_sampling_reset: AtomicU64,

    exposure_dedupe_set: RwLock<AHashSet<ExposureSamplingKey>>,
    last_exposure_dedupe_reset: AtomicU64,

    global_configs: Arc<GlobalConfigs>,
}

impl ExposureSampling {
    pub fn new(sdk_key: &str) -> Self {
        let now = Utc::now().timestamp_millis() as u64;

        Self {
            spec_sampling_set: RwLock::from(AHashSet::default()),
            last_spec_sampling_reset: AtomicU64::from(now),

            exposure_dedupe_set: RwLock::from(AHashSet::default()),
            last_exposure_dedupe_reset: AtomicU64::from(now),

            global_configs: GlobalConfigs::get_instance(sdk_key),
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

        let extra_info = exposure.get_extra_exposure_info_ref();
        if self.should_sample_based_on_evaluation(extra_info) {
            return EvtSamplingDecision::ForceSampled;
        }

        if self.should_sample_first_time_exposure(&expo_sampling_key) {
            return EvtSamplingDecision::ForceSampled;
        }

        let sampling_rate = self
            .get_special_case_sampling_rate(exposure)
            .or_else(|| extra_info.and_then(|info| info.sampling_rate));

        let is_sampled = expo_sampling_key.is_sampled(sampling_rate);

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

    fn should_dedupe_exposure(&self, sampling_key: &ExposureSamplingKey) -> bool {
        let mut dedupe_set = write_lock_or_return!(TAG, self.exposure_dedupe_set, false);
        if dedupe_set.contains(sampling_key) {
            return true;
        }

        dedupe_set.insert(sampling_key.clone());
        false
    }

    fn should_sample_based_on_evaluation(&self, extra_info: Option<&ExtraExposureInfo>) -> bool {
        let exposure_info = match extra_info {
            Some(exposure_info) => exposure_info,
            None => return false,
        };

        if exposure_info.forward_all_exposures == Some(true) {
            return true;
        }

        if exposure_info.has_seen_analytical_gates == Some(true) {
            return true;
        }

        false
    }

    fn should_sample_first_time_exposure(&self, exposure: &ExposureSamplingKey) -> bool {
        let sampling_key: SpecAndRuleHashTuple = (exposure.spec_name_hash, exposure.rule_id_hash);
        if self.sample_key_exists(&sampling_key) {
            return false;
        }

        match self
            .spec_sampling_set
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut sampling_map) => {
                sampling_map.insert(sampling_key);
            }
            None => {
                log_e!(TAG, "Failed to acquire write lock for spec sampling set");
            }
        }

        true
    }

    fn try_reset_spec_sampling_set(&self) {
        let now = Utc::now().timestamp_millis() as u64;
        let last_sampling_reset = self.last_spec_sampling_reset.load(Ordering::Relaxed);
        let mut sampling_map = write_lock_or_noop!(TAG, self.spec_sampling_set);

        let has_expired = now - last_sampling_reset > SAMPLING_TTL_MS;
        let is_full = sampling_map.len() > SAMPLING_MAX_KEYS;

        if has_expired || is_full {
            log_d!(
                TAG,
                "Resetting spec sampling set. has_expired: {:?}, is_full: {:?}",
                has_expired,
                is_full
            );
            sampling_map.clear();
            self.last_spec_sampling_reset.store(now, Ordering::Relaxed);
        }
    }

    fn try_reset_exposure_dedupe_set(&self) {
        let now = Utc::now().timestamp_millis() as u64;
        let last_dedupe_reset = self.last_exposure_dedupe_reset.load(Ordering::Relaxed);
        let mut dedupe_map = match self
            .exposure_dedupe_set
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(map) => map,
            None => {
                log_e!(TAG, "Failed to acquire write lock for exposure dedupe set");
                return;
            }
        };

        let has_expired = now - last_dedupe_reset > SAMPLING_TTL_MS;
        let is_full = dedupe_map.len() > SAMPLING_MAX_KEYS;

        if has_expired || is_full {
            log_d!(
                TAG,
                "Resetting exposure dedupe set. has_expired: {:?}, is_full: {:?}",
                has_expired,
                is_full
            );
            dedupe_map.clear();
            self.last_exposure_dedupe_reset
                .store(now, Ordering::Relaxed);
        }
    }

    fn sample_key_exists(&self, key: &SpecAndRuleHashTuple) -> bool {
        match self
            .spec_sampling_set
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(map) => map.contains(key),
            None => false,
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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ExposureSamplingKey {
    pub spec_name_hash: u64,
    pub rule_id_hash: u64,
    pub user_values_hash: u64,
    pub additional_hash: u64,
}

impl ExposureSamplingKey {
    pub fn new(evaluation: Option<&BaseEvaluation>, user: &UserData, additional_hash: u64) -> Self {
        let spec_name_hash = evaluation.as_ref().map_or(0, |e| e.name.hash);
        let rule_id_hash = evaluation.as_ref().map_or(0, |e| e.rule_id.hash);

        let user_values_hash = user.create_user_values_hash();

        Self {
            spec_name_hash,
            rule_id_hash,
            user_values_hash,
            additional_hash,
        }
    }

    pub fn is_sampled(&self, sampling_rate: Option<u64>) -> bool {
        let sampling_rate = match sampling_rate {
            Some(rate) => rate,
            None => return true, // without a sampling rate, we should sample
        };

        let final_hash =
            self.spec_name_hash ^ self.rule_id_hash ^ self.user_values_hash ^ self.additional_hash;

        final_hash % sampling_rate == 0
    }
}
