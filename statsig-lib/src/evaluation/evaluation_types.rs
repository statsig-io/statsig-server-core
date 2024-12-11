use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_false(v: &bool) -> bool {
    *v == false
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryExposure {
    pub gate: String,
    pub gate_value: String,
    #[serde(rename = "ruleID")]
    pub rule_id: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct BaseEvaluation {
    pub name: String,
    pub rule_id: String,
    pub secondary_exposures: Vec<SecondaryExposure>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GateEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub id_type: String,
    pub value: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfigEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub id_type: String,
    pub value: Value,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: String,
    pub is_device_based: bool,

    pub passed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExperimentEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub id_type: String,
    pub value: Value,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: String,
    pub is_device_based: bool,

    #[serde(skip_serializing_if = "is_false")]
    pub is_in_layer: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_parameters: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experiment_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_in_experiment: Option<bool>,
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

    pub value: Value,

    // The 'group' field is identical to 'rule_id'. See group_name instead.
    pub group: String,
    pub is_device_based: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_experiment_active: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_in_experiment: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocated_experiment_name: Option<String>,
    pub explicit_parameters: Vec<String>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
}
