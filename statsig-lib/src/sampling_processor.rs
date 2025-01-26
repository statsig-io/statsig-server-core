use crate::hashing::HashUtil;
use crate::hashset_with_ttl::HashSetWithTTL;
use crate::spec_store::SpecStore;
use crate::statsig_user_internal::StatsigUserInternal;
use crate::{StatsigErr, StatsigRuntime};
use std::sync::Arc;
use tokio::time::Duration;

const TTL_IN_SECONDS: u64 = 60;

#[allow(dead_code)] // TODO(@dloomb): Remove this when sampling is completed
pub struct SamplingProcessor {
    sampling_key_set: HashSetWithTTL,
    spec_store: Arc<SpecStore>,
    hashing: Arc<HashUtil>,
}

impl SamplingProcessor {
    pub fn new(
        statsig_runtime: &Arc<StatsigRuntime>,
        spec_store: &Arc<SpecStore>,
        hashing: Arc<HashUtil>,
    ) -> Self {
        let sampling_key_set =
            HashSetWithTTL::new(statsig_runtime, Duration::from_secs(TTL_IN_SECONDS));

        Self {
            sampling_key_set,
            spec_store: spec_store.clone(),
            hashing,
        }
    }

    pub async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.sampling_key_set.shutdown().await;
        Ok(())
    }

    /// compute sampling key for gate / experiment / dynamic config
    #[allow(dead_code)] // TODO(@dloomb): Remove this when sampling is completed
    fn compute_sampling_key_for_gate_or_config(
        &self,
        name: &str,
        rule_id: &str,
        value: &bool,
        user: &StatsigUserInternal,
    ) -> String {
        let user_key = self.compute_user_key(user);
        format!("n:{};u:{};r:{};v:{}", name, user_key, rule_id, value)
    }

    /// compute sampling key for layers
    #[allow(dead_code)] // TODO(@dloomb): Remove this when sampling is completed
    fn compute_sampling_key_for_layer(
        &self,
        layer_name: &str,
        experiment_name: &str,
        parameter_name: &str,
        rule_id: &str,
        user: &StatsigUserInternal,
    ) -> String {
        let user_key = self.compute_user_key(user);
        format!(
            "n:{};e:{};p:{};u:{};r:{}",
            layer_name, experiment_name, parameter_name, user_key, rule_id
        )
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
                    user_key.push_str(&format!("{}:{};", key, string_value));
                }
            }
        };

        user_key
    }
}
