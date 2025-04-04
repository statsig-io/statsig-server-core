use crate::evaluation::evaluation_types::AnyEvaluation;
use crate::global_configs::GlobalConfigs;
use crate::hashing::HashUtil;
use crate::hashset_with_ttl::HashSetWithTTL;
use crate::statsig_user_internal::StatsigUserInternal;
use crate::{DynamicValue, StatsigErr, StatsigRuntime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;

const SPECIAL_CASE_RULES: [&str; 3] = ["disabled", "default", ""];
const TTL_IN_SECONDS: u64 = 60;

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SamplingStatus {
    Logged,
    Dropped,
    #[default]
    None,
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SamplingMode {
    On,
    Shadow,
    #[default]
    None,
}

#[derive(Default)]
pub struct SamplingDecision {
    pub should_send_exposure: bool,
    pub sampling_rate: Option<u64>,
    pub sampling_status: SamplingStatus,
    pub sampling_mode: SamplingMode,
}

impl SamplingDecision {
    pub fn new(
        should_send_exposure: bool,
        sampling_rate: Option<u64>,
        sampling_status: SamplingStatus,
        sampling_mode: SamplingMode,
    ) -> Self {
        Self {
            should_send_exposure,
            sampling_rate,
            sampling_status,
            sampling_mode,
        }
    }

    pub fn force_logged() -> Self {
        Self {
            should_send_exposure: true,
            sampling_rate: None,
            sampling_status: SamplingStatus::None,
            sampling_mode: SamplingMode::None,
        }
    }
}

pub struct SamplingProcessor {
    sampling_key_set: HashSetWithTTL,
    hashing: Arc<HashUtil>,
    global_configs: Arc<GlobalConfigs>,
}

impl SamplingProcessor {
    pub fn new(
        statsig_runtime: &Arc<StatsigRuntime>,
        hashing: Arc<HashUtil>,
        sdk_key: &str,
    ) -> Self {
        let sampling_key_set =
            HashSetWithTTL::new(statsig_runtime, Duration::from_secs(TTL_IN_SECONDS));

        Self {
            sampling_key_set,
            hashing,
            global_configs: GlobalConfigs::get_instance(sdk_key),
        }
    }

    pub async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.sampling_key_set.shutdown().await;
        Ok(())
    }

    pub fn get_sampling_decision_and_details(
        &self,
        user: &StatsigUserInternal,
        eval_result: Option<&AnyEvaluation>,
        parameter_name_for_layer: Option<&str>,
    ) -> SamplingDecision {
        let eval_result = match eval_result {
            Some(result) => result,
            None => return SamplingDecision::force_logged(),
        };

        if self.should_skip_sampling(eval_result, &user.statsig_environment) {
            return SamplingDecision::force_logged();
        }

        let base_eval_res = eval_result.get_base_result();
        let sampling_ttl_set_key = format!("{}_{}", base_eval_res.name, base_eval_res.rule_id);

        if !self
            .sampling_key_set
            .contains(&sampling_ttl_set_key)
            .unwrap_or(false)
        {
            let _ = self.sampling_key_set.add(sampling_ttl_set_key);
            return SamplingDecision::force_logged();
        }

        let sampling_mode = self.get_sampling_mode();
        let sampling_exposure_key =
            self.compute_sampling_exposure_key(eval_result, user, parameter_name_for_layer);

        let (should_send_exposures, sampling_rate) =
            self.evaluate_exposure_sending(eval_result, &sampling_exposure_key);

        let sampling_log_status = match sampling_rate {
            None => SamplingStatus::None, // No sampling rate, no status
            Some(_) if should_send_exposures => SamplingStatus::Logged,
            Some(_) => SamplingStatus::Dropped,
        };

        match sampling_mode {
            SamplingMode::On => SamplingDecision::new(
                should_send_exposures,
                sampling_rate,
                sampling_log_status,
                SamplingMode::On,
            ),
            SamplingMode::Shadow => SamplingDecision::new(
                true,
                sampling_rate,
                sampling_log_status,
                SamplingMode::Shadow,
            ),
            _ => SamplingDecision::force_logged(),
        }
    }

    // -------------------------
    //   Utils For Generating Sampling Related Exposure Key
    // -------------------------

    fn compute_sampling_exposure_key(
        &self,
        eval_result: &AnyEvaluation,
        user: &StatsigUserInternal,
        parameter_name_for_layer: Option<&str>,
    ) -> String {
        let base_eval_res = eval_result.get_base_result();

        match eval_result {
            AnyEvaluation::Layer(eval) => self.compute_sampling_key_for_layer(
                &base_eval_res.name,
                eval.allocated_experiment_name.as_deref().unwrap_or("null"),
                parameter_name_for_layer.unwrap_or("null"),
                &base_eval_res.rule_id,
                user,
            ),
            _ => self.compute_sampling_key_for_gate_or_config(
                &base_eval_res.name,
                &base_eval_res.rule_id,
                &eval_result.get_gate_bool_value(),
                user,
            ),
        }
    }

    /// compute sampling key for gate / experiment / dynamic config
    fn compute_sampling_key_for_gate_or_config(
        &self,
        name: &str,
        rule_id: &str,
        value: &bool,
        user: &StatsigUserInternal,
    ) -> String {
        let user_key = self.compute_user_key(user);
        format!("n:{name};u:{user_key};r:{rule_id};v:{value}")
    }

    /// compute sampling key for layers
    fn compute_sampling_key_for_layer(
        &self,
        layer_name: &str,
        experiment_name: &str,
        parameter_name: &str,
        rule_id: &str,
        user: &StatsigUserInternal,
    ) -> String {
        let user_key = self.compute_user_key(user);
        format!("n:{layer_name};e:{experiment_name};p:{parameter_name};u:{user_key};r:{rule_id}")
    }

    fn compute_user_key(&self, user: &StatsigUserInternal) -> String {
        let user_data = &user.user_data;

        let mut user_key = format!(
            "u:{};",
            user_data
                .user_id
                .as_ref()
                .and_then(|id| id.string_value.as_deref())
                .unwrap_or("")
        );

        if let Some(custom_ids) = user_data.custom_ids.as_ref() {
            for (key, val) in custom_ids {
                if let Some(string_value) = &val.string_value {
                    user_key.push_str(&format!("{key}:{string_value};"));
                }
            }
        };

        user_key
    }

    // -------------------------
    //   Other Helper Functions
    // -------------------------

    /// Returns a tuple:
    /// - `bool`: Whether exposures should be sent.
    /// - `Option<u64>`: The sampling rate used for the decision (if applicable).
    fn evaluate_exposure_sending(
        &self,
        eval_result: &AnyEvaluation,
        sampling_exposure_key: &String,
    ) -> (bool, Option<u64>) {
        let eval_base_res = eval_result.get_base_result();
        let special_case_sampling_rate = self.get_special_case_sampling_rate();

        if SPECIAL_CASE_RULES.contains(&eval_base_res.rule_id.as_str())
            && special_case_sampling_rate.is_some()
        {
            if let Some(special_rate) = special_case_sampling_rate {
                let should_send_exposures =
                    self.is_hash_in_sampling_rate(sampling_exposure_key, special_rate);
                return (should_send_exposures, Some(special_rate));
            }
        }

        if let Some(rate) = eval_base_res
            .sampling_info
            .as_ref()
            .and_then(|info| info.sampling_rate)
        {
            let should_send_exposures = self.is_hash_in_sampling_rate(sampling_exposure_key, rate);

            return (should_send_exposures, Some(rate));
        }

        (true, None) // default to true, always send exposures, do NOT sample
    }

    fn should_skip_sampling(
        &self,
        eval_result: &AnyEvaluation,
        env: &Option<HashMap<String, DynamicValue>>,
    ) -> bool {
        let sampling_mode = self.get_sampling_mode();

        if matches!(sampling_mode, SamplingMode::None) {
            return true;
        }

        let sampling_info = eval_result.get_base_result().sampling_info.as_ref();

        if sampling_info
            .and_then(|info| info.forward_all_exposures)
            .unwrap_or(false)
        {
            return true;
        }

        if sampling_info
            .and_then(|info| info.has_seen_analytical_gates)
            .unwrap_or(false)
        {
            return true;
        }

        // skip sampling if env is not in production
        if env
            .as_ref()
            .and_then(|e| e.get("tier"))
            .and_then(|tier| tier.string_value.as_deref())
            != Some("production")
        {
            return true;
        }

        let rule_id = &eval_result.get_base_result().rule_id;
        if rule_id.ends_with(":override") || rule_id.ends_with(":id_override") {
            return true;
        }

        false
    }

    fn is_hash_in_sampling_rate(&self, key: &String, sampling_rate: u64) -> bool {
        let hash_value = self.hashing.sha256_to_u64(key);
        hash_value % sampling_rate == 0
    }

    fn get_sampling_mode(&self) -> SamplingMode {
        self.global_configs
            .get_sdk_config_value("sampling_mode")
            .and_then(|mode| mode.string_value)
            .as_deref()
            .map_or(SamplingMode::None, |mode_str| match mode_str {
                "on" => SamplingMode::On,
                "shadow" => SamplingMode::Shadow,
                _ => SamplingMode::None,
            })
    }

    fn get_special_case_sampling_rate(&self) -> Option<u64> {
        self.global_configs
            .get_sdk_config_value("special_case_sampling_rate")
            .and_then(|v| v.float_value)
            .map(|rate| rate as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::evaluation_types::{BaseEvaluation, GateEvaluation};
    use crate::{SpecStore, SpecsSource, SpecsUpdate, StatsigUser};
    use serde_json::Value;
    use std::fs;
    use std::sync::LazyLock;

    static GATE: LazyLock<GateEvaluation> = LazyLock::new(|| GateEvaluation {
        base: BaseEvaluation {
            name: "publish_to_all".to_string(),
            rule_id: "rule_id".to_string(),
            secondary_exposures: vec![],
            sampling_info: Default::default(),
        },
        id_type: String::new(),
        value: false,
    });

    fn create_mock_user() -> StatsigUserInternal {
        let mut custom_ids = HashMap::new();
        custom_ids.insert("k1".to_string(), "v1".to_string());
        custom_ids.insert("k2".to_string(), "v2".to_string());

        StatsigUserInternal {
            user_data: StatsigUser::with_custom_ids(custom_ids),
            statsig_environment: Some(HashMap::from([(
                "tier".to_string(),
                DynamicValue {
                    string_value: Some("development".to_string()),
                    ..Default::default()
                },
            )])),
        }
    }

    fn create_mock_evaluation_result() -> AnyEvaluation<'static> {
        AnyEvaluation::FeatureGate(&GATE)
    }

    #[test]
    fn test_should_skip_sampling() {
        let file_path = "tests/data/dcs_with_sdk_configs.json";
        let file_content = fs::read_to_string(file_path).expect("Unable to read file");
        let json_data: Value = serde_json::from_str(&file_content).expect("Unable to parse JSON");

        // Create the mocked SpecStore with sdk configs
        let specs_update = SpecsUpdate {
            data: json_data.to_string(),
            source: SpecsSource::Network,
            received_at: 2000,
        };

        let spec_store = SpecStore::default();
        spec_store
            .set_values(specs_update)
            .expect("Set Specstore failed");

        // initialize sampling processor
        let runtime = StatsigRuntime::get_runtime();
        let hashing = Arc::new(HashUtil::new());
        let processor = SamplingProcessor::new(&runtime, hashing, "");

        let mut test_user = create_mock_user();
        let mock_evaluation_res = create_mock_evaluation_result();

        // Should skip sampling in a non-production environment
        let should_skip_sample =
            processor.should_skip_sampling(&mock_evaluation_res, &test_user.statsig_environment);
        assert!(should_skip_sample);

        test_user.statsig_environment = Some(HashMap::from([(
            "tier".to_string(),
            DynamicValue {
                string_value: Some("production".to_string()),
                ..Default::default()
            },
        )]));
        // should not skip sampling in a production environment
        let should_skip_sample_2 =
            processor.should_skip_sampling(&mock_evaluation_res, &test_user.statsig_environment);
        assert!(!should_skip_sample_2);
    }
}
