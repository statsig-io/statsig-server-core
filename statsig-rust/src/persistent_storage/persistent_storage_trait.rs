use std::{collections::HashMap, sync::Weak};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::evaluation::dynamic_string::DynamicString;
use crate::event_logging::event_logger::EventLogger;
use crate::{
    evaluation::evaluation_types::{ExperimentEvaluation, LayerEvaluation},
    statsig_type_factories::{extract_from_experiment_evaluation, make_layer},
    statsig_types::{Experiment, Layer},
    unwrap_or_return,
    user::StatsigUserInternal,
    EvaluationDetails, SecondaryExposure,
};

pub type UserPersistedValues = HashMap<String, StickyValues>;

pub trait PersistentStorage: Send + Sync {
    fn load(&self, key: String) -> Option<UserPersistedValues>;
    fn save(&self, key: &str, config_name: &str, data: StickyValues);
    fn delete(&self, key: &str, config_name: &str);
}

pub fn get_persistent_storage_key(user: &StatsigUserInternal, id_type: &String) -> Option<String> {
    let dyn_str_id_type = DynamicString::from(id_type.clone());
    user.get_unit_id(&dyn_str_id_type).map(|id| {
        let mut id_str = "";
        if let Some(id) = id.string_value.as_ref().map(|s| &s.value) {
            id_str = id;
        }

        format!("{}:{}", id_str, id_type)
    })
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StickyValues {
    pub value: bool,
    pub json_value: Option<HashMap<String, Value>>,
    pub rule_id: Option<String>,
    pub group_name: Option<String>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub config_delegate: Option<String>,
    pub explicit_parameters: Option<Vec<String>>,
    pub time: Option<u64>,
    pub config_version: Option<u32>,
}

pub fn make_layer_from_sticky_value(
    name: &str,
    user: &StatsigUserInternal,
    evaluation: LayerEvaluation,
    sticky_value: StickyValues,
    event_logger_ptr: Option<Weak<EventLogger>>,
    disable_exposure: bool,
) -> Layer {
    let details = EvaluationDetails {
        reason: "Persisted".to_owned(),
        lcut: sticky_value.time,
        received_at: Some(Utc::now().timestamp_millis() as u64),
    };
    make_layer(
        user.to_loggable(),
        name,
        Some(evaluation),
        details,
        event_logger_ptr,
        sticky_value.config_version,
        disable_exposure,
        None,
    )
}

pub fn make_sticky_value_from_layer(layer: &Layer) -> Option<StickyValues> {
    let layer_evaluation = unwrap_or_return!(layer.__evaluation.as_ref(), None);
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
        config_version: layer.__version,
    })
}

pub fn make_experiment_from_sticky_value(
    evaluation: ExperimentEvaluation,
    sticky_value: StickyValues,
) -> Experiment {
    let name = evaluation.base.name.clone();
    let maybe_evaluation = Some(evaluation);
    let (value, rule_id, id_type, group_name) =
        extract_from_experiment_evaluation(&maybe_evaluation);
    let details = EvaluationDetails {
        reason: "Persisted".to_owned(),
        lcut: sticky_value.time,
        received_at: Some(Utc::now().timestamp_millis() as u64),
    };
    Experiment {
        name,
        value,
        rule_id,
        id_type,
        group_name,
        details,
        __evaluation: maybe_evaluation,
        __version: sticky_value.config_version,
        __override_config_name: None,
    }
}

pub fn make_sticky_value_from_experiment(experiment: &Experiment) -> Option<StickyValues> {
    let experiment_evaluation = unwrap_or_return!(&experiment.__evaluation, None);
    Some(StickyValues {
        value: true, // For sticky value, if it's being saved, it should always be true
        json_value: Some(experiment_evaluation.value.clone()),
        rule_id: Some(experiment_evaluation.base.rule_id.clone()),
        group_name: experiment_evaluation.group_name.clone(),
        secondary_exposures: experiment_evaluation.base.secondary_exposures.clone(),
        undelegated_secondary_exposures: experiment_evaluation
            .undelegated_secondary_exposures
            .clone(),
        config_delegate: None,
        explicit_parameters: experiment_evaluation.explicit_parameters.clone(),
        time: experiment.details.lcut,
        config_version: experiment.__version,
    })
}
