use super::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_types::is_false;
use crate::interned_string::InternedString;
use crate::SecondaryExposure;
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

    pub id_type: InternedString,
    pub value: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfigEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

    pub id_type: InternedString,
    pub value: DynamicReturnable,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: InternedString,
    pub is_device_based: bool,

    pub passed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExperimentEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

    pub id_type: InternedString,
    pub value: DynamicReturnable,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: InternedString,
    pub is_device_based: bool,

    #[serde(skip_serializing_if = "is_false")]
    pub is_in_layer: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_parameters: Option<Vec<InternedString>>,

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
pub enum AnyConfigEvaluationV2 {
    DynamicConfig(DynamicConfigEvaluationV2),
    Experiment(ExperimentEvaluationV2),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LayerEvaluationV2 {
    #[serde(flatten)]
    pub base: BaseEvaluationV2,

    pub value: DynamicReturnable,

    pub id_type: InternedString,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: InternedString,
    pub is_device_based: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<InternedString>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experiment_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_in_experiment: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocated_experiment_name: Option<InternedString>,
    pub explicit_parameters: Vec<InternedString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undelegated_secondary_exposures: Option<Vec<InternedString>>,
}
