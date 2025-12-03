use crate::evaluation::dynamic_string::DynamicString;
use crate::evaluation::evaluation_types::BaseEvaluation;
use crate::interned_string::InternedString;
use crate::specs_response::explicit_params::ExplicitParameters;
use crate::{log_e, DynamicReturnable, StatsigUser};
use crate::{
    statsig_types::{Experiment, Layer},
    unwrap_or_return, SecondaryExposure,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub type UserPersistedValues = HashMap<String, StickyValues>;

pub trait PersistentStorage: Send + Sync {
    fn load(&self, key: String) -> Option<UserPersistedValues>;
    fn save(&self, key: &str, config_name: &str, data: StickyValues);
    fn delete(&self, key: &str, config_name: &str);

    fn get_values_for_user(
        &self,
        user: &StatsigUser,
        id_type: &str,
    ) -> Option<UserPersistedValues> {
        let key = get_persistent_storage_key(user, id_type)?;
        self.load(key)
    }
}

pub fn get_persistent_storage_key(user: &StatsigUser, id_type: &str) -> Option<String> {
    let dyn_str_id_type = DynamicString::from(id_type.to_string());
    user.get_unit_id(&dyn_str_id_type).map(|id| {
        let mut id_str = "";
        if let Some(id) = id.string_value.as_ref().map(|s| &s.value) {
            id_str = id;
        }

        format!("{id_str}:{id_type}")
    })
}

const TAG: &str = "PersistentStorageTrait";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StickyValues {
    pub value: bool,
    pub json_value: Option<DynamicReturnable>,
    pub rule_id: Option<InternedString>,
    pub group_name: Option<InternedString>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub config_delegate: Option<InternedString>,
    pub explicit_parameters: Option<ExplicitParameters>,
    #[serde(deserialize_with = "deserialize_safe_timestamp")]
    pub time: Option<u64>,
    pub config_version: Option<u32>,
}

fn deserialize_safe_timestamp<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    if value.is_null() {
        return Ok(None);
    }

    if let Value::Number(num) = value {
        if let Some(timestamp) = num.as_u64() {
            return Ok(Some(timestamp));
        }

        if let Some(timestamp) = num.as_i64() {
            return Ok(Some(timestamp as u64));
        }

        if let Some(timestamp) = num.as_f64() {
            return Ok(Some(timestamp as u64));
        }
    }

    log_e!(TAG, "Non-numeric 'timestamp' field");

    Ok(None)
}

pub fn make_sticky_value_from_layer(layer: &Layer) -> Option<StickyValues> {
    let layer_evaluation = unwrap_or_return!(layer.__evaluation.as_ref(), None);
    let config_version = extract_config_version(&layer_evaluation.base);

    Some(StickyValues {
        value: true,
        json_value: Some(layer_evaluation.value.clone()),
        rule_id: Some(layer_evaluation.base.rule_id.clone()),
        group_name: layer_evaluation.group_name.clone(),
        secondary_exposures: layer_evaluation.base.secondary_exposures.clone(),
        undelegated_secondary_exposures: layer_evaluation.undelegated_secondary_exposures.clone(),
        config_delegate: layer_evaluation.allocated_experiment_name.clone(),
        explicit_parameters: Some(layer_evaluation.explicit_parameters.clone()),
        time: layer.details.lcut,
        config_version,
    })
}

pub fn make_sticky_value_from_experiment(experiment: &Experiment) -> Option<StickyValues> {
    let evaluation = unwrap_or_return!(&experiment.__evaluation, None);
    let config_version = extract_config_version(&evaluation.base);

    Some(StickyValues {
        value: true, // For sticky value, if it's being saved, it should always be true
        json_value: Some(evaluation.value.clone()),
        rule_id: Some(evaluation.base.rule_id.clone()),
        group_name: evaluation.group_name.clone(),
        secondary_exposures: evaluation.base.secondary_exposures.clone(),
        undelegated_secondary_exposures: evaluation.undelegated_secondary_exposures.clone(),
        config_delegate: None,
        explicit_parameters: evaluation.explicit_parameters.as_ref().cloned(),
        time: experiment.details.lcut,
        config_version,
    })
}

fn extract_config_version(evaluation: &BaseEvaluation) -> Option<u32> {
    evaluation
        .exposure_info
        .as_ref()
        .and_then(|info| info.version)
}
