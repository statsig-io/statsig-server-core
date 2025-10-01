use super::dynamic_returnable::DynamicReturnable;
use super::evaluation_types::ExtraExposureInfo;
use super::evaluation_types_v2::{
    BaseEvaluationV2, DynamicConfigEvaluationV2, ExperimentEvaluationV2, GateEvaluationV2,
    LayerEvaluationV2,
};
use crate::evaluation::evaluation_types::{
    BaseEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
    SecondaryExposure,
};
use crate::hashing::{HashAlgorithm, HashUtil};
use crate::interned_string::InternedString;

#[derive(Default, Debug)]
pub struct EvaluatorResult<'a> {
    pub name: Option<&'a InternedString>,
    pub bool_value: bool,
    pub unsupported: bool,
    pub is_experiment_group: bool,
    pub is_experiment_active: bool,
    pub is_in_layer: bool,
    pub is_in_experiment: bool,
    pub id_type: Option<InternedString>,
    pub json_value: Option<DynamicReturnable>,
    pub rule_id: Option<&'a InternedString>,
    pub rule_id_suffix: Option<&'static str>,
    pub group_name: Option<InternedString>,
    pub explicit_parameters: Option<&'a Vec<InternedString>>,
    pub config_delegate: Option<InternedString>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub override_reason: Option<&'a str>,
    pub version: Option<u32>,
    pub sampling_rate: Option<u64>,
    pub forward_all_exposures: Option<bool>,
    pub override_config_name: Option<&'a str>,
    pub has_seen_analytical_gates: Option<bool>,
}

pub fn result_to_gate_eval(gate_name: &str, result: &mut EvaluatorResult) -> GateEvaluation {
    GateEvaluation {
        base: result_to_base_eval(gate_name, result),
        id_type: result.id_type.take(),
        value: result.bool_value,
    }
}

pub fn result_to_gate_eval_v2(
    gate_name: &str,
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
) -> GateEvaluationV2 {
    GateEvaluationV2 {
        base: result_to_base_eval_v2(gate_name, result, hashing),
        id_type: result.id_type.take(),
        value: result.bool_value,
    }
}

pub fn result_to_experiment_eval(
    experiment_name: &str,
    spec_entity: Option<&str>,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if spec_entity.is_none_or(|s| s == "experiment") {
        is_experiment_active = Some(result.is_experiment_active);
        is_user_in_experiment = Some(result.is_experiment_group);
    }

    ExperimentEvaluation {
        base: result_to_base_eval(experiment_name, result),
        id_type: Some(id_type),
        is_device_based,
        value: get_json_value(result),
        is_in_layer: result.is_in_layer,
        group_name: result.group_name.take(),
        explicit_parameters: result.explicit_parameters.cloned(),
        is_experiment_active,
        is_user_in_experiment,
        undelegated_secondary_exposures: std::mem::take(
            &mut result.undelegated_secondary_exposures,
        ),
    }
}

pub fn result_to_experiment_eval_v2(
    experiment_name: &str,
    spec_entity: Option<&str>,
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
) -> ExperimentEvaluationV2 {
    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if let Some(spec_entity) = spec_entity {
        if spec_entity == "experiment" {
            is_experiment_active = Some(result.is_experiment_active);
            is_user_in_experiment = Some(result.is_experiment_group);
        }
    }

    ExperimentEvaluationV2 {
        base: result_to_base_eval_v2(experiment_name, result, hashing),
        id_type: Some(id_type),
        is_device_based,
        value: get_json_value(result),
        is_in_layer: result.is_in_layer,
        group_name: result.group_name.take(),
        explicit_parameters: result.explicit_parameters.cloned(),
        is_experiment_active,
        is_user_in_experiment,
        undelegated_secondary_exposures: result.undelegated_secondary_exposures.clone(),
    }
}

pub fn eval_result_to_experiment_eval(
    experiment_name: &str,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    ExperimentEvaluation {
        base: result_to_base_eval(experiment_name, result),
        id_type: Some(id_type),
        is_device_based,
        value: get_json_value(result),
        is_in_layer: result.is_in_layer,
        group_name: result.group_name.take(),
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

    if let Some(config_delegate) = result.config_delegate.take() {
        if !config_delegate.is_empty() {
            allocated_experiment_name = Some(config_delegate.clone());
            is_experiment_active = Some(result.is_experiment_active);
            is_user_in_experiment = Some(result.is_experiment_group);
        }
    }

    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());
    let undelegated_sec_expos = std::mem::take(&mut result.undelegated_secondary_exposures);

    LayerEvaluation {
        base: result_to_base_eval(layer_name, result),
        value: get_json_value(result),
        is_device_based,
        group_name: result.group_name.take(),
        is_experiment_active,
        is_user_in_experiment,
        allocated_experiment_name,
        explicit_parameters: result.explicit_parameters.cloned().unwrap_or_default(),
        undelegated_secondary_exposures: Some(undelegated_sec_expos.unwrap_or_default()),
        id_type: Some(id_type),
    }
}

pub fn result_to_layer_eval_v2(
    layer_name: &str,
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
) -> LayerEvaluationV2 {
    let mut undelegated_secondary_exposures = Vec::new();

    if let Some(u) = &result.undelegated_secondary_exposures {
        for exposure in u {
            let key = format!(
                "{}:{}:{}",
                exposure.gate,
                exposure.gate_value,
                exposure.rule_id.as_str()
            );
            let hash = hashing.hash(&key, &HashAlgorithm::Djb2);
            undelegated_secondary_exposures.push(InternedString::from_string(hash));
        }
    }

    let mut allocated_experiment_name = None;
    let mut is_experiment_active = None;
    let mut is_user_in_experiment = None;

    if let Some(config_delegate) = result.config_delegate.take() {
        if !config_delegate.is_empty() {
            allocated_experiment_name = Some(config_delegate.clone());
            is_experiment_active = Some(result.is_experiment_active);
            is_user_in_experiment = Some(result.is_experiment_group);
        }
    }

    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    LayerEvaluationV2 {
        base: result_to_base_eval_v2(layer_name, result, hashing),
        value: get_json_value(result),
        is_device_based,
        group_name: result.group_name.take(),
        is_experiment_active,
        is_user_in_experiment,
        allocated_experiment_name,
        explicit_parameters: result.explicit_parameters.cloned().unwrap_or_default(),
        undelegated_secondary_exposures: Some(undelegated_secondary_exposures),
        id_type: Some(id_type),
    }
}

pub fn result_to_dynamic_config_eval(
    dynamic_config_name: &str,
    result: &mut EvaluatorResult,
) -> DynamicConfigEvaluation {
    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    DynamicConfigEvaluation {
        base: result_to_base_eval(dynamic_config_name, result),
        id_type: Some(id_type),
        is_device_based,
        value: get_json_value(result),
        passed: result.bool_value,
    }
}

pub fn result_to_dynamic_config_eval_v2(
    dynamic_config_name: &str,
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
) -> DynamicConfigEvaluationV2 {
    let (id_type, is_device_based) = get_id_type_info(result.id_type.as_ref());

    DynamicConfigEvaluationV2 {
        base: result_to_base_eval_v2(dynamic_config_name, result, hashing),
        id_type: Some(id_type),
        is_device_based,
        value: get_json_value(result),
        passed: result.bool_value,
    }
}

fn get_id_type_info(id_type: Option<&InternedString>) -> (InternedString, bool) {
    let id_type = id_type.cloned().unwrap_or_default();
    let is_device_based = id_type == "stableID" || id_type == "stableid";
    (id_type, is_device_based)
}

fn get_json_value(result: &mut EvaluatorResult) -> DynamicReturnable {
    result
        .json_value
        .take()
        .unwrap_or_else(DynamicReturnable::empty)
}

// todo: remove when 'QueuedExposure' does not use `BaseEvaluation`
fn get_exposure_name_if_not_hashed(
    possibly_hashed_name: &str,
    exposure_name: Option<&InternedString>,
) -> InternedString {
    let exposure_name = exposure_name.unwrap_or(InternedString::empty_ref());
    if possibly_hashed_name == exposure_name.as_str() {
        exposure_name.clone()
    } else {
        InternedString::from_str_ref(possibly_hashed_name)
    }
}

fn result_to_base_eval(spec_name: &str, result: &mut EvaluatorResult) -> BaseEvaluation {
    let rule_id = create_suffixed_rule_id(result.rule_id, result.rule_id_suffix);

    let exposure_info = ExtraExposureInfo {
        sampling_rate: result.sampling_rate,
        forward_all_exposures: result.forward_all_exposures,
        has_seen_analytical_gates: result.has_seen_analytical_gates,
        override_config_name: result.override_config_name.map(|s| s.to_string()),
        version: result.version,
    };

    let name = get_exposure_name_if_not_hashed(spec_name, result.name);

    BaseEvaluation {
        name,
        rule_id,
        secondary_exposures: std::mem::take(&mut result.secondary_exposures),
        exposure_info: Some(exposure_info),
    }
}

fn result_to_base_eval_v2(
    spec_name: &str,
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
) -> BaseEvaluationV2 {
    let mut exposures = Vec::new();

    for exposure in &result.secondary_exposures {
        let key = format!(
            "{}:{}:{}",
            exposure.gate,
            exposure.gate_value,
            exposure.rule_id.as_str()
        );
        let hash = hashing.hash(&key, &HashAlgorithm::Djb2);
        exposures.push(hash.clone());
    }

    let rule_id = create_suffixed_rule_id(result.rule_id, result.rule_id_suffix);

    BaseEvaluationV2 {
        name: spec_name.to_string(),
        rule_id,
        secondary_exposures: exposures,
    }
}

fn create_suffixed_rule_id(
    rule_id: Option<&InternedString>,
    suffix: Option<&str>,
) -> InternedString {
    let id_arc = match &rule_id {
        Some(rule_id) => rule_id.as_str(),
        None => "",
    };

    match &suffix {
        Some(suffix) => InternedString::from_str_parts(&[id_arc, ":", suffix]),
        None => rule_id.cloned().unwrap_or_default(),
    }
}
