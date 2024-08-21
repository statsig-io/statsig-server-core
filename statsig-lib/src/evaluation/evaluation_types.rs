use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub id_type: String,
    pub secondary_exposures: Vec<SecondaryExposure>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GateEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub value: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfigEvaluation {
    #[serde(flatten)]
    pub base: BaseEvaluation,

    pub value: Value,
    pub group: String,
    pub is_device_based: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExperimentEvaluation {
    #[serde(flatten)]
    pub base: DynamicConfigEvaluation,

    pub group_name: Option<String>,
    pub is_experiment_active: bool,
    pub is_user_in_experiment: bool,
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
    pub base: DynamicConfigEvaluation,

    pub allocated_experiment_name: String,
    pub explicit_parameters: Vec<String>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
}
