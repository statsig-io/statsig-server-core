use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::EventLogger;
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::statsig_user_internal::StatsigUserInternal;
use crate::SamplingProcessor;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Weak;

pub fn make_feature_gate(
    name: &str,
    evaluation: Option<GateEvaluation>,
    details: EvaluationDetails,
    version: Option<u32>,
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
        __version: version,
    }
}

fn extract_from_experiment_evaluation(
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
    version: Option<u32>,
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
        __version: version,
    }
}

pub fn make_experiment(
    name: &str,
    evaluation: Option<ExperimentEvaluation>,
    details: EvaluationDetails,
    version: Option<u32>,
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
        __version: version,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_layer(
    user: &StatsigUserInternal,
    name: &str,
    evaluation: Option<LayerEvaluation>,
    details: EvaluationDetails,
    event_logger_ptr: Option<Weak<EventLogger>>,
    version: Option<u32>,
    disable_exposure: bool,
    sampling_processor: Option<Weak<SamplingProcessor>>,
) -> Layer {
    let (value, rule_id, group_name, allocated_experiment_name) = match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            e.group_name.clone(),
            e.allocated_experiment_name.clone(),
        ),
        None => (HashMap::new(), "default".into(), None, None),
    };

    Layer {
        name: name.to_string(),
        rule_id,
        details: details.clone(),
        group_name,
        allocated_experiment_name,
        __value: value,
        __evaluation: evaluation,
        __user: user.clone(),
        __event_logger_ptr: event_logger_ptr,
        __version: version,
        __disable_exposure: disable_exposure,
        __sampling_processor: sampling_processor,
    }
}
