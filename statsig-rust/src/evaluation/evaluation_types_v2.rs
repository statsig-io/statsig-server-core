use super::dynamic_returnable::DynamicReturnable;
use crate::gcir::gcir_formatter::GCIRHashable;
use crate::hashing::opt_bool_to_hashable;
use crate::interned_string::InternedString;
use crate::{
    evaluation::evaluation_types::is_false, specs_response::explicit_params::ExplicitParameters,
};
use crate::{hashing, SecondaryExposure};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct BaseEvaluationV2 {
    pub name: String,
    pub rule_id: InternedString,
    pub secondary_exposures: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GateEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,
    pub value: bool,
}

impl GCIRHashable for GateEvaluationV2 {
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
pub struct DynamicConfigEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_type: Option<InternedString>,
    pub value: DynamicReturnable,

    pub is_device_based: bool,

    pub passed: bool,
}

impl GCIRHashable for DynamicConfigEvaluationV2 {
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
pub struct ExperimentEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

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

impl GCIRHashable for ExperimentEvaluationV2 {
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
pub enum AnyConfigEvaluationV2 {
    DynamicConfig(DynamicConfigEvaluationV2),
    Experiment(ExperimentEvaluationV2),
}

impl GCIRHashable for AnyConfigEvaluationV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        match self {
            AnyConfigEvaluationV2::DynamicConfig(eval) => eval.create_hash(name),
            AnyConfigEvaluationV2::Experiment(eval) => eval.create_hash(name),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LayerEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

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
    pub undelegated_secondary_exposures: Option<Vec<InternedString>>,
}

impl GCIRHashable for LayerEvaluationV2 {
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
                undelegated_secondary_exposure_hashes.push(exposure.hash);
            }
        }
        hash_array.push(hashing::hash_one(undelegated_secondary_exposure_hashes));

        hashing::hash_one(hash_array)
    }
}

fn hash_secondary_exposures(exposures: &Vec<String>) -> u64 {
    let mut secondary_exposure_hashes = Vec::new();
    for exposure in exposures {
        secondary_exposure_hashes.push(hashing::ahash_str(exposure));
    }
    hashing::hash_one(secondary_exposure_hashes)
}
