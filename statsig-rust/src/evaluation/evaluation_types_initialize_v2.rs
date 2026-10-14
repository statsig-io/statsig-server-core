use crate::{
    gcir::gcir_formatter::GCIRHashable,
    hashing::{self, opt_bool_to_hashable},
    interned_string::InternedString,
    specs_response::explicit_params::ExplicitParameters,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct BaseEvaluationInitV2 {
    #[serde(rename = "r")]
    pub rule_id: Option<InternedString>,
    #[serde(rename = "s")]
    pub secondary_exposures: Option<Vec<InternedString>>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct GateEvaluationInitV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationInitV2,

    #[serde(rename = "i")]
    pub id_type: Option<InternedString>,
    #[serde(rename = "v")]
    pub value: Option<bool>,
}

impl GCIRHashable for GateEvaluationInitV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![
            name.hash,
            match self.value {
                Some(true) => 1,
                _ => 0,
            },
            match self.base.rule_id {
                Some(ref rule_id) => rule_id.hash,
                None => 0,
            },
            hash_secondary_exposures(&self.base.secondary_exposures),
        ];
        hashing::hash_one(hash_array)
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfigEvaluationInitV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationInitV2,

    #[serde(rename = "i")]
    pub id_type: Option<InternedString>,
    #[serde(rename = "v")]
    pub value: InternedString,
    #[serde(rename = "p")]
    pub passed: bool,
}

impl GCIRHashable for DynamicConfigEvaluationInitV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![
            name.hash,
            self.value.hash,
            match self.base.rule_id {
                Some(ref rule_id) => rule_id.hash,
                None => 0,
            },
            hash_secondary_exposures(&self.base.secondary_exposures),
            if self.passed { 1 } else { 0 },
        ];
        hashing::hash_one(hash_array)
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct ExperimentEvaluationInitV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationInitV2,

    #[serde(rename = "i")]
    pub id_type: Option<InternedString>,
    #[serde(rename = "v")]
    pub value: InternedString,

    #[serde(rename = "gn")]
    pub group_name: Option<InternedString>,

    #[serde(rename = "ea")]
    pub is_experiment_active: Option<bool>,

    #[serde(rename = "ue")]
    pub is_user_in_experiment: Option<bool>,
}

impl GCIRHashable for ExperimentEvaluationInitV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let hash_array = vec![
            name.hash,
            self.value.hash,
            match self.base.rule_id {
                Some(ref rule_id) => rule_id.hash,
                None => 0,
            },
            hash_secondary_exposures(&self.base.secondary_exposures),
            self.group_name.as_ref().map_or(0, |g| g.hash),
            opt_bool_to_hashable(&self.is_experiment_active),
            opt_bool_to_hashable(&self.is_user_in_experiment),
        ];

        hashing::hash_one(hash_array)
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AnyConfigEvaluationInitV2 {
    DynamicConfig(DynamicConfigEvaluationInitV2),
    Experiment(ExperimentEvaluationInitV2),
}

impl GCIRHashable for AnyConfigEvaluationInitV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        match self {
            AnyConfigEvaluationInitV2::DynamicConfig(eval) => eval.create_hash(name),
            AnyConfigEvaluationInitV2::Experiment(eval) => eval.create_hash(name),
        }
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone)]
pub struct LayerEvaluationInitV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationInitV2,

    #[serde(rename = "v")]
    pub value: InternedString,

    #[serde(rename = "i")]
    pub id_type: Option<InternedString>,

    #[serde(rename = "gn")]
    pub group_name: Option<InternedString>,

    #[serde(rename = "ea")]
    pub is_experiment_active: Option<bool>,

    #[serde(rename = "ue")]
    pub is_user_in_experiment: Option<bool>,

    #[serde(rename = "ae")]
    pub allocated_experiment_name: Option<InternedString>,
    #[serde(rename = "ep")]
    pub explicit_parameters: Option<ExplicitParameters>,
    #[serde(rename = "us")]
    pub undelegated_secondary_exposures: Option<Vec<InternedString>>,
}

impl GCIRHashable for LayerEvaluationInitV2 {
    fn create_hash(&self, name: &InternedString) -> u64 {
        let mut hash_array = vec![
            name.hash,
            self.value.hash,
            match self.base.rule_id {
                Some(ref rule_id) => rule_id.hash,
                None => 0,
            },
            hash_secondary_exposures(&self.base.secondary_exposures),
            self.group_name.as_ref().map_or(0, |g| g.hash),
            opt_bool_to_hashable(&self.is_experiment_active),
            opt_bool_to_hashable(&self.is_user_in_experiment),
            self.allocated_experiment_name
                .as_ref()
                .map_or(0, |n| n.hash),
        ];
        let mut explicit_params_hashes = Vec::new();
        if let Some(explicit_parameters) = &self.explicit_parameters {
            for value in explicit_parameters.to_vec_interned() {
                explicit_params_hashes.push(value.hash);
            }
        }
        hash_array.push(hash_secondary_exposures(
            &self.undelegated_secondary_exposures,
        ));

        hashing::hash_one(hash_array)
    }
}

fn hash_secondary_exposures(exposures: &Option<Vec<InternedString>>) -> u64 {
    let mut secondary_exposure_hashes = Vec::new();
    if let Some(exposures) = exposures {
        for exposure in exposures {
            secondary_exposure_hashes.push(exposure.hash);
        }
    }
    hashing::hash_one(secondary_exposure_hashes)
}
