use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::EventLogger;
use crate::interned_string::InternedString;
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::user::StatsigUserLoggable;
use std::sync::Weak;

pub fn make_feature_gate(
    name: &str,
    evaluation: Option<GateEvaluation>,
    details: EvaluationDetails,
) -> FeatureGate {
    let (value, rule_id, id_type) = match &evaluation {
        Some(e) => (
            e.value,
            e.base.rule_id.clone(),
            match e.id_type {
                Some(ref id_type) => id_type.unperformant_to_string(),
                None => "".into(),
            },
        ),
        None => (false, InternedString::default_rule_id(), String::new()),
    };

    FeatureGate {
        name: name.to_string(),
        rule_id: rule_id.unperformant_to_string(),
        id_type,
        value,
        details,
        __evaluation: evaluation,
    }
}

pub fn extract_from_experiment_evaluation(
    evaluation: &Option<ExperimentEvaluation>,
) -> (
    DynamicReturnable,
    InternedString,
    String,
    Option<String>,
    bool,
) {
    match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            match e.id_type {
                Some(ref id_type) => id_type.unperformant_to_string(),
                None => "".into(),
            },
            e.group_name.as_ref().map(|g| g.unperformant_to_string()),
            e.is_experiment_active.unwrap_or(false),
        ),
        None => (
            DynamicReturnable::empty(),
            InternedString::default_rule_id(),
            String::new(),
            None,
            false,
        ),
    }
}

pub fn make_dynamic_config(
    name: &str,
    evaluation: Option<DynamicConfigEvaluation>,
    details: EvaluationDetails,
) -> DynamicConfig {
    let (value, rule_id, id_type) = match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            match e.id_type {
                Some(ref id_type) => id_type.unperformant_to_string(),
                None => "".into(),
            },
        ),
        None => (
            DynamicReturnable::empty(),
            InternedString::default_rule_id(),
            String::new(),
        ),
    };

    DynamicConfig {
        name: name.to_string(),
        rule_id: rule_id.unperformant_to_string(),
        id_type,
        value: value.get_json().unwrap_or_default(),
        details,
        __evaluation: evaluation,
    }
}

pub fn make_experiment(
    name: &str,
    evaluation: Option<ExperimentEvaluation>,
    details: EvaluationDetails,
) -> Experiment {
    let (value, rule_id, id_type, group_name, is_experiment_active) =
        extract_from_experiment_evaluation(&evaluation);

    Experiment {
        name: name.to_string(),
        rule_id: rule_id.unperformant_to_string(),
        id_type,
        value: value.get_json().unwrap_or_default(),
        details: details.clone(),
        group_name,
        is_experiment_active,
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
    let (
        value,
        rule_id,
        group_name,
        allocated_experiment_name,
        id_type,
        is_experiment_active,
        parameter_rule_ids,
    ) = match &evaluation {
        Some(e) => (
            e.value.clone(),
            e.base.rule_id.clone(),
            e.group_name.as_ref().map(|g| g.unperformant_to_string()),
            e.allocated_experiment_name
                .as_ref()
                .map(|g| g.unperformant_to_string()),
            match e.id_type {
                Some(ref id_type) => id_type.unperformant_to_string(),
                None => "".into(),
            },
            e.is_experiment_active.unwrap_or(false),
            e.parameter_rule_ids.clone(),
        ),
        None => (
            DynamicReturnable::empty(),
            InternedString::default_rule_id(),
            None,
            None,
            "".into(),
            false,
            None,
        ),
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
        rule_id: rule_id.unperformant_to_string(),
        id_type,
        details: details.clone(),
        group_name,
        allocated_experiment_name,
        is_experiment_active,
        __value: value.get_json().unwrap_or_default(),
        __evaluation: evaluation,
        __user: user,
        __event_logger_ptr: event_logger_ptr,
        __disable_exposure: disable_exposure,
        __version: version,
        __parameter_rule_ids: parameter_rule_ids,
    }
}
