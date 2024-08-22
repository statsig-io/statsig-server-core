use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_types::{BaseEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation, SecondaryExposure};
use crate::spec_types::Spec;
use serde_json::Value;

#[derive(Default, Debug)]
pub struct EvaluatorResult<'a> {
    pub bool_value: bool,
    pub unsupported: bool,
    pub is_experiment_group: bool,
    pub json_value: Option<&'a DynamicReturnable>,
    pub rule_id: Option<&'a String>,
    pub group_name: Option<&'a String>,
    pub explicit_parameters: Option<&'a Vec<String>>,
    pub config_delegate: Option<&'a String>,
    pub secondary_exposures: Vec<SecondaryExposure>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
}

impl<'a> EvaluatorResult<'a> {
    pub fn finalize_secondary_exposures(&mut self) {
        if self.secondary_exposures.is_empty() {
            return
        }

        if self.undelegated_secondary_exposures.is_some() {
            // this shouldn't happen
            return;
        }

        self.undelegated_secondary_exposures = Some(self.secondary_exposures.clone())
    }
}

pub fn result_to_gate_eval(
    gate_name: &str,
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> GateEvaluation {
    GateEvaluation {
        base: result_to_base_eval(gate_name, spec, result),
        value: result.bool_value,
    }
}

pub fn result_to_experiment_eval(
    experiment_name: &str,
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> ExperimentEvaluation {
    ExperimentEvaluation {
        base: result_to_dynamic_config_eval(experiment_name, spec, result),

        group_name: result.group_name.cloned(),
        is_experiment_active: spec.is_active.unwrap_or(false),
        is_user_in_experiment: result.is_experiment_group,
    }
}

pub fn result_to_layer_eval(
    layer_name: &str,
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> LayerEvaluation {
    let mut undelegated_secondary_exposures = Vec::new();
    if let Some(u) = &mut result.undelegated_secondary_exposures {
        undelegated_secondary_exposures = std::mem::take(u);
    }

    LayerEvaluation {
        base: result_to_dynamic_config_eval(layer_name, spec, result),

        allocated_experiment_name: result.config_delegate.cloned().unwrap_or_default(),
        explicit_parameters: result.explicit_parameters.cloned().unwrap_or_default(),
        undelegated_secondary_exposures: Some(undelegated_secondary_exposures),
    }
}

pub fn result_to_dynamic_config_eval(
    dynamic_config_name: &str,
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> DynamicConfigEvaluation {
    let mut value = Value::Null;
    if let Some(v) = result.json_value {
        value = v.value.clone();
    }

    DynamicConfigEvaluation {
        base: result_to_base_eval(dynamic_config_name, spec, result),

        value,
        group: result.group_name.cloned().unwrap_or_default(),
        is_device_based: false,
    }
}

fn result_to_base_eval(
    spec_name: &str,
    spec: &Spec,
    result: &mut EvaluatorResult,
) -> BaseEvaluation {
    let secondary_exposures = std::mem::take(&mut result.secondary_exposures);
    BaseEvaluation {
        id_type: spec.id_type.clone(),
        name: spec_name.to_string(),
        rule_id: result.rule_id.cloned().unwrap_or_default(),
        secondary_exposures,
    }
}
