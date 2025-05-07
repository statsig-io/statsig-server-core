use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::EventLogger;
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::user::StatsigUserLoggable;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Weak;

pub fn make_feature_gate(
    name: &str,
    evaluation: Option<GateEvaluation>,
    details: EvaluationDetails,
) -> FeatureGate {
    let (value, rule_id, id_type) = match &evaluation {
        Some(e) => (e.value, e.base.rule_id.clone(), e.id_type.clone()),
        None => (false, "default".into(), String::new()),
    };

    FeatureGate {
        name: name.to_string(),
        rule_id,
        id_type,
        value,
        details,
        __evaluation: evaluation,
    }
}

pub fn extract_from_experiment_evaluation(
    evaluation: &Option<ExperimentEvaluation>,
) -> (HashMap<String, Value>, String, String, Option<String>) {
    match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            e.id_type.clone(),
            e.group_name.clone(),
        ),
        None => (HashMap::new(), "default".into(), String::new(), None),
    }
}

pub fn make_dynamic_config(
    name: &str,
    evaluation: Option<DynamicConfigEvaluation>,
    details: EvaluationDetails,
) -> DynamicConfig {
    let (value, rule_id, id_type) = match &evaluation {
        Some(e) => (e.value.clone(), e.base.rule_id.clone(), e.id_type.clone()),
        None => (HashMap::new(), "default".into(), String::new()),
    };

    DynamicConfig {
        name: name.to_string(),
        rule_id,
        id_type,
        value,
        details,
        __evaluation: evaluation,
    }
}

pub fn make_experiment(
    name: &str,
    evaluation: Option<ExperimentEvaluation>,
    details: EvaluationDetails,
) -> Experiment {
    let (value, rule_id, id_type, group_name) = extract_from_experiment_evaluation(&evaluation);

    Experiment {
        name: name.to_string(),
        rule_id,
        id_type,
        value,
        details: details.clone(),
        group_name,
        __evaluation: evaluation,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_layer(
    user: StatsigUserLoggable,
    name: &str,
    evaluation: Option<LayerEvaluation>,
    details: EvaluationDetails,
    event_logger_ptr: Option<Weak<EventLogger>>,
    disable_exposure: bool,
) -> Layer {
    let (value, rule_id, group_name, allocated_experiment_name, id_type) = match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            e.group_name.clone(),
            e.allocated_experiment_name.clone(),
            e.id_type.clone(),
        ),
        None => (HashMap::new(), "default".into(), None, None, "".into()),
    };

    let mut version = None;
    if let Some(exposure_info) = evaluation.as_ref().map(|e| &e.base.exposure_info) {
        version = exposure_info
            .as_ref()
            .map(|info| info.version)
            .unwrap_or_default();
    }

    Layer {
        name: name.to_string(),
        rule_id,
        id_type,
        details: details.clone(),
        group_name,
        allocated_experiment_name,
        __value: value,
        __evaluation: evaluation,
        __user: user,
        __event_logger_ptr: event_logger_ptr,
        __disable_exposure: disable_exposure,
        __version: version,
    }
}
