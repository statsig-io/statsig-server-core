use crate::evaluation::evaluation_types::{
    BaseEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, ExposureSamplingInfo,
    GateEvaluation, LayerEvaluation, SecondaryExposure,
};
use crate::spec_types::Spec;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct EvaluatorResult<'a> {
    pub bool_value: bool,
    pub unsupported: bool,
    pub is_experiment_group: bool,
    pub is_experiment_active: bool,
    pub is_in_layer: bool,
    pub id_type: Option<&'a String>,
    pub json_value: Option<HashMap<String, Value>>,
    pub rule_id: Option<&'a String>,
    pub rule_id_suffix: Option<String>,
    pub group_name: Option<&'a String>,
    pub explicit_parameters: Option<&'a Vec<String>>,
    pub config_delegate: Option<&'a String>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub override_reason: Option<&'a str>,
    pub version: Option<u32>,
    pub sampling_rate: Option<u64>,
    pub forward_all_exposures: Option<bool>,
    pub override_config_name: Option<String>,
    pub has_seen_analytical_gates: Option<bool>,
}

pub fn result_to_gate_eval(gate_name: &str, result: &mut EvaluatorResult) -> GateEvaluation {
    GateEvaluation {
        base: result_to_base_eval(gate_name, result),
        id_type: result.id_type.cloned().unwrap_or_default(),
        value: result.bool_value,
    }
}

pub fn result_to_experiment_eval(
    experiment_name: &str,
    spec: Option<&Spec>,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type);

    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if let Some(spec) = spec {
        if spec.entity == "experiment" {
            is_experiment_active = Some(result.is_experiment_active);
            is_user_in_experiment = Some(result.is_experiment_group);
        }
    }

    ExperimentEvaluation {
        base: result_to_base_eval(experiment_name, result),
        id_type,
        group: result.rule_id.cloned().unwrap_or_default(),
        is_device_based,
        value: get_json_value(result),
        is_in_layer: result.is_in_layer,
        group_name: result.group_name.cloned(),
        explicit_parameters: result.explicit_parameters.cloned(),
        is_experiment_active,
        is_user_in_experiment,
        undelegated_secondary_exposures: std::mem::take(
            &mut result.undelegated_secondary_exposures,
        ),
    }
}

pub fn eval_result_to_experiment_eval(
    experiment_name: &str,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type);

    ExperimentEvaluation {
        base: result_to_base_eval(experiment_name, result),
        id_type,
        group: result.rule_id.cloned().unwrap_or_default(),
        is_device_based,
        value: get_json_value(result),
        is_in_layer: result.is_in_layer,
        group_name: result.group_name.cloned(),
        explicit_parameters: result.explicit_parameters.cloned(),
        is_experiment_active: Some(result.is_experiment_active),
        is_user_in_experiment: Some(result.is_experiment_group),
        undelegated_secondary_exposures: std::mem::take(
            &mut result.undelegated_secondary_exposures,
        ),
    }
}

pub fn result_to_layer_eval(layer_name: &str, result: &mut EvaluatorResult) -> LayerEvaluation {
    let mut allocated_experiment_name = None;
    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if let Some(config_delegate) = result.config_delegate {
        if !config_delegate.is_empty() {
            allocated_experiment_name = Some(config_delegate.clone());
            is_experiment_active = Some(result.is_experiment_active);
            is_user_in_experiment = Some(result.is_experiment_group);
        }
    }

    let (id_type, is_device_based) = get_id_type_info(result.id_type);
    let undelegated_sec_expos = std::mem::take(&mut result.undelegated_secondary_exposures);

    LayerEvaluation {
        base: result_to_base_eval(layer_name, result),
        group: result.rule_id.cloned().unwrap_or_default(),
        value: get_json_value(result),
        is_device_based,
        group_name: result.group_name.cloned(),
        is_experiment_active,
        is_user_in_experiment,
        allocated_experiment_name,
        explicit_parameters: result.explicit_parameters.cloned().unwrap_or_default(),
        undelegated_secondary_exposures: Some(undelegated_sec_expos.unwrap_or_default()),
        id_type,
    }
}

pub fn result_to_dynamic_config_eval(
    dynamic_config_name: &str,
    result: &mut EvaluatorResult,
) -> DynamicConfigEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type);

    DynamicConfigEvaluation {
        base: result_to_base_eval(dynamic_config_name, result),
        id_type,
        is_device_based,
        value: get_json_value(result),
        group: result.rule_id.cloned().unwrap_or_default(),
        passed: result.bool_value,
    }
}

fn get_id_type_info(id_type: Option<&String>) -> (String, bool) {
    let id_type = id_type.cloned().unwrap_or_default();
    let is_device_based = id_type == "stableID" || id_type == "stableid";
    (id_type, is_device_based)
}

fn get_json_value(result: &EvaluatorResult) -> HashMap<String, Value> {
    result.json_value.clone().unwrap_or_default()
}

fn result_to_base_eval(spec_name: &str, result: &mut EvaluatorResult) -> BaseEvaluation {
    let rule_id = match result.rule_id {
        Some(rule_id) => rule_id.clone(),
        None => String::new(),
    };

    let result_rule_id = match &result.rule_id_suffix {
        Some(suffix) => format!("{rule_id}:{suffix}"),
        None => rule_id.clone(),
    };

    let sampling_info = ExposureSamplingInfo {
        sampling_rate: result.sampling_rate,
        forward_all_exposures: result.forward_all_exposures,
        has_seen_analytical_gates: result.has_seen_analytical_gates,
    };

    BaseEvaluation {
        name: spec_name.to_string(),
        rule_id: result_rule_id.clone(),
        secondary_exposures: std::mem::take(&mut result.secondary_exposures),
        sampling_info: Some(sampling_info),
    }
}
