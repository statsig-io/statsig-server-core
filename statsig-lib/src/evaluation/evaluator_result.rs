use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_types::{
    BaseEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
    SecondaryExposure,
};
use crate::spec_types::Spec;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct EvaluatorResult<'a> {
    pub bool_value: bool,
    pub unsupported: bool,
    pub is_experiment_group: bool,
    pub is_experiment_active: bool,
    pub is_in_layer: bool,
    pub id_type: Option<&'a String>,
    pub json_value: Option<&'a DynamicReturnable>,
    pub rule_id: Option<&'a String>,
    pub group_name: Option<&'a String>,
    pub explicit_parameters: Option<&'a Vec<String>>,
    pub config_delegate: Option<&'a String>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
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
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    let mut value = Value::Null;
    if let Some(v) = result.json_value {
        value = v.value.clone();
    }

    let (id_type, is_device_based) = get_id_type_info(result.id_type);

    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if spec.entity == "experiment" {
        is_experiment_active = Some(result.is_experiment_active);
        is_user_in_experiment = Some(result.is_experiment_group);
    }

    ExperimentEvaluation {
        base: result_to_base_eval(experiment_name, result),
        id_type,
        group: result.rule_id.cloned().unwrap_or_default(),
        is_device_based,
        value,
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
    let mut value = Value::Null;
    if let Some(v) = result.json_value {
        value = v.value.clone();
    }

    let (id_type, is_device_based) = get_id_type_info(result.id_type);

    DynamicConfigEvaluation {
        base: result_to_base_eval(dynamic_config_name, result),
        id_type,
        is_device_based,
        value,
        group: result.rule_id.cloned().unwrap_or_default(),
    }
}

fn get_id_type_info(id_type: Option<&String>) -> (String, bool) {
    let id_type = id_type.cloned().unwrap_or_default();
    let is_device_based = id_type == "stableID" || id_type == "stableid";
    (id_type, is_device_based)
}

fn get_json_value(result: &EvaluatorResult) -> Value {
    result
        .json_value
        .map(|v| v.value.clone())
        .unwrap_or(Value::Null)
}

fn result_to_base_eval(spec_name: &str, result: &mut EvaluatorResult) -> BaseEvaluation {
    let mut exposures = Vec::new();
    let mut seen = HashSet::new();

    for exposure in result.secondary_exposures.iter() {
        let key = format!(
            "{}:{}:{}",
            exposure.gate, exposure.gate_value, exposure.rule_id
        );

        if !seen.contains(&key) {
            seen.insert(key);
            exposures.push(exposure.clone());
        }
    }

    BaseEvaluation {
        name: spec_name.to_string(),
        rule_id: result.rule_id.cloned().unwrap_or_default(),
        secondary_exposures: exposures,
    }
}
