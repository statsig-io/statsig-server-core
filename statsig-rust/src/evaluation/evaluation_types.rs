use std::collections::HashMap;

use super::dynamic_returnable::DynamicReturnable;
use crate::{
    evaluation::secondary_exposure_key::SecondaryExposureKey,
    gcir::gcir_formatter::GCIRHashable,
    hashing::{self, opt_bool_to_hashable},
    interned_string::InternedString,
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

impl GCIRHashable for SecondaryExposure {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![name.hash, self.gate_value.hash, self.rule_id.hash];
        hashing::hash_one(hash_array)
    }
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

impl GCIRHashable for GateEvaluation {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![
            name.hash,
            self.value as u64,
            self.base.rule_id.hash,
            hash_secondary_exposures(&self.base.secondary_exposures),
        ];
        hashing::hash_one(hash_array)
    }
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

impl GCIRHashable for DynamicConfigEvaluation {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![
            name.hash,
            self.value.get_hash(),
            self.base.rule_id.hash,
            hash_secondary_exposures(&self.base.secondary_exposures),
            self.passed as u64,
        ];
        hashing::hash_one(hash_array)
    }
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

impl GCIRHashable for ExperimentEvaluation {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let mut hash_array = vec![
            name.hash,
            self.value.get_hash(),
            self.base.rule_id.hash,
            hash_secondary_exposures(&self.base.secondary_exposures),
            self.is_in_layer as u64,
        ];
        let mut explicit_params_hashes = Vec::new();
        if let Some(explicit_parameters) = &self.explicit_parameters {
            for value in explicit_parameters.to_vec_interned() {
                explicit_params_hashes.push(value.hash);
            }
        }
        hash_array.push(hashing::hash_one(explicit_params_hashes));
        hash_array.push(self.group_name.as_ref().map_or(0, |g| g.hash));
        hash_array.push(opt_bool_to_hashable(&self.is_experiment_active));
        hash_array.push(opt_bool_to_hashable(&self.is_user_in_experiment));

        hashing::hash_one(hash_array)
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AnyConfigEvaluation {
    DynamicConfig(DynamicConfigEvaluation),
    Experiment(ExperimentEvaluation),
}

impl GCIRHashable for AnyConfigEvaluation {
    fn create_hash(&self, name: &InternedString) -> u64 {
        match self {
            AnyConfigEvaluation::DynamicConfig(eval) => eval.create_hash(name),
            AnyConfigEvaluation::Experiment(eval) => eval.create_hash(name),
        }
    }
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

impl GCIRHashable for LayerEvaluation {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let mut hash_array = vec![
            name.hash,
            self.value.get_hash(),
            self.base.rule_id.hash,
            hash_secondary_exposures(&self.base.secondary_exposures),
            self.group_name.as_ref().map_or(0, |g| g.hash),
            opt_bool_to_hashable(&self.is_experiment_active),
            opt_bool_to_hashable(&self.is_user_in_experiment),
            self.allocated_experiment_name
                .as_ref()
                .map_or(0, |n| n.hash),
        ];
        let mut explicit_params_hashes = Vec::new();
        for value in self.explicit_parameters.to_vec_interned() {
            explicit_params_hashes.push(value.hash);
        }
        hash_array.push(hashing::hash_one(explicit_params_hashes));
        let mut undelegated_secondary_exposure_hashes = Vec::new();
        if let Some(undelegated_secondary_exposures) = &self.undelegated_secondary_exposures {
            for exposure in undelegated_secondary_exposures {
                undelegated_secondary_exposure_hashes.push(exposure.create_hash(&exposure.gate));
            }
        }
        hash_array.push(hashing::hash_one(undelegated_secondary_exposure_hashes));
        if let Some(parameter_rule_ids) = &self.parameter_rule_ids {
            let mut param_rule_ids_hash = Vec::new();
            for (param_name, rule_id) in parameter_rule_ids {
                param_rule_ids_hash.push(param_name.hash);
                param_rule_ids_hash.push(rule_id.hash);
            }
            hash_array.push(hashing::hash_one(param_rule_ids_hash));
        }

        hashing::hash_one(hash_array)
    }
}

fn hash_secondary_exposures(exposures: &Vec<SecondaryExposure>) -> u64 {
    let mut secondary_exposure_hashes = Vec::new();
    for exposure in exposures {
        secondary_exposure_hashes.push(exposure.create_hash(&exposure.gate));
    }
    hashing::hash_one(secondary_exposure_hashes)
}
