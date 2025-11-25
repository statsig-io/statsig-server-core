use std::collections::HashMap;

use super::dynamic_returnable::DynamicReturnable;
use crate::{
    evaluation::secondary_exposure_key::SecondaryExposureKey, interned_string::InternedString,
    specs_response::explicit_params::ExplicitParameters,
};

use serde::{Deserialize, Serialize};

pub fn is_false(v: &bool) -> bool {
    !(*v)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryExposure {
    pub gate: InternedString,
    pub gate_value: InternedString,
    #[serde(rename = "ruleID")]
    pub rule_id: InternedString,
}

impl SecondaryExposure {
    pub fn get_dedupe_key(&self) -> String {
        let mut key = String::new();
        key += &self.gate;
        key += "|";
        key += &self.gate_value;
        key += "|";
        key += self.rule_id.as_str();
        key
    }
}

impl From<&SecondaryExposure> for SecondaryExposureKey {
    fn from(val: &SecondaryExposure) -> Self {
        SecondaryExposureKey {
            gate_name_hash: val.gate.hash,
            rule_id_hash: val.rule_id.hash,
            gate_value_hash: val.gate_value.hash,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ExtraExposureInfo {
    pub sampling_rate: Option<u64>,
    pub forward_all_exposures: Option<bool>,
    pub has_seen_analytical_gates: Option<bool>,
    pub override_config_name: Option<InternedString>,
    pub version: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BaseEvaluation {
    pub name: InternedString,
    pub rule_id: InternedString,
    pub secondary_exposures: Vec<SecondaryExposure>,

    #[serde(skip_serializing)]
    pub(crate) exposure_info: Option<ExtraExposureInfo>,
}

pub enum AnyEvaluation<'a> {
    FeatureGate(&'a GateEvaluation),
    DynamicConfig(&'a DynamicConfigEvaluation),
    Experiment(&'a ExperimentEvaluation),
    Layer(&'a LayerEvaluation),
}

impl AnyEvaluation<'_> {
    pub fn get_base_result(&self) -> &BaseEvaluation {
        match self {
            AnyEvaluation::FeatureGate(gate) => &gate.base,
            AnyEvaluation::DynamicConfig(config) => &config.base,
            AnyEvaluation::Experiment(experiment) => &experiment.base,
            AnyEvaluation::Layer(layer) => &layer.base,
        }
    }

    pub fn get_gate_bool_value(&self) -> bool {
        match self {
            AnyEvaluation::FeatureGate(eval) => eval.value,
            _ => false, // return false for all other types
        }
    }
}

impl<'a> From<&'a LayerEvaluation> for AnyEvaluation<'a> {
    fn from(layer_eval: &'a LayerEvaluation) -> Self {
        AnyEvaluation::Layer(layer_eval)
    }
}

impl<'a> From<&'a GateEvaluation> for AnyEvaluation<'a> {
    fn from(gate_eval: &'a GateEvaluation) -> Self {
        AnyEvaluation::FeatureGate(gate_eval)
    }
}

impl<'a> From<&'a ExperimentEvaluation> for AnyEvaluation<'a> {
    fn from(experiment_eval: &'a ExperimentEvaluation) -> Self {
        AnyEvaluation::Experiment(experiment_eval)
    }
}

impl<'a> From<&'a DynamicConfigEvaluation> for AnyEvaluation<'a> {
    fn from(dynamic_config_evalation: &'a DynamicConfigEvaluation) -> Self {
        AnyEvaluation::DynamicConfig(dynamic_config_evalation)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GateEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,
    pub value: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfigEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,
    pub value: DynamicReturnable,

    pub is_device_based: bool,

    pub passed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExperimentEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,
    pub value: DynamicReturnable,

    pub is_device_based: bool,

    #[serde(skip_serializing_if = "is_false")]
    pub is_in_layer: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_parameters: Option<ExplicitParameters>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<InternedString>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experiment_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_in_experiment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AnyConfigEvaluation {
    DynamicConfig(DynamicConfigEvaluation),
    Experiment(ExperimentEvaluation),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LayerEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub value: DynamicReturnable,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,

    pub is_device_based: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<InternedString>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experiment_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_in_experiment: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocated_experiment_name: Option<InternedString>,
    pub explicit_parameters: ExplicitParameters,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_rule_ids: Option<HashMap<InternedString, InternedString>>,
}
