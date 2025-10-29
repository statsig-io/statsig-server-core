use crate::interned_string::InternedString;
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AnyConfigEvaluationInitV2 {
    DynamicConfig(DynamicConfigEvaluationInitV2),
    Experiment(ExperimentEvaluationInitV2),
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
    pub explicit_parameters: Option<Vec<InternedString>>,
    #[serde(rename = "us")]
    pub undelegated_secondary_exposures: Option<Vec<InternedString>>,
}
