use crate::evaluation::evaluation_types::{
    BaseEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
    SecondaryExposure,
};
use crate::spec_types::Spec;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
    }
}

pub fn result_to_layer_eval(layer_name: &str, result: &mut EvaluatorResult) -> LayerEvaluation {
    let mut undelegated_secondary_exposures = Vec::new();

    if let Some(u) = &mut result.undelegated_secondary_exposures {
        undelegated_secondary_exposures = std::mem::take(u);
    }

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

    let (_, is_device_based) = get_id_type_info(result.id_type);

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
        undelegated_secondary_exposures: Some(undelegated_secondary_exposures),
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
    let mut exposures = Vec::new();
    let mut seen = HashSet::new();

    for exposure in &result.secondary_exposures {
        let key = format!(
            "{}:{}:{}",
            exposure.gate, exposure.gate_value, exposure.rule_id
        );

        if !seen.contains(&key) {
            seen.insert(key);
            exposures.push(exposure.clone());
        }
    }

    let rule_id = match result.rule_id {
        Some(rule_id) => rule_id.clone(),
        None => String::new(),
    };

    let result_rule_id = match &result.rule_id_suffix {
        Some(suffix) => format!("{rule_id}:{suffix}"),
        None => rule_id.clone(),
    };

    BaseEvaluation {
        name: spec_name.to_string(),
        rule_id: result_rule_id.clone(),
        secondary_exposures: exposures,
        sampling_rate: result.sampling_rate,
        forward_all_exposures: result.forward_all_exposures,
    }
}
