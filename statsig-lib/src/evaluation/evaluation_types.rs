use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_false(v: &bool) -> bool {
    !(*v)
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
    #[serde(skip_serializing)]
    pub sampling_rate: Option<u64>,
    #[serde(skip_serializing)]
    pub forward_all_exposures: Option<bool>,
}

pub enum AnyEvaluation {
    FeatureGate(GateEvaluation),
    DynamicConfig(DynamicConfigEvaluation),
    Experiment(ExperimentEvaluation),
    Layer(LayerEvaluation),
}

impl AnyEvaluation {
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
            AnyEvaluation::FeatureGate(eval ) => eval.value,
            _ => false, // return false for all other types
        }
    }
}

impl From<LayerEvaluation> for AnyEvaluation {
    fn from(layer_eval: LayerEvaluation) -> Self {
        AnyEvaluation::Layer(layer_eval)
    }
}

impl From<GateEvaluation> for AnyEvaluation {
    fn from(gate_eval: GateEvaluation) -> Self {
        AnyEvaluation::FeatureGate(gate_eval)
    }
}

impl From<ExperimentEvaluation> for AnyEvaluation {
    fn from(experiment_eval: ExperimentEvaluation) -> Self {
        AnyEvaluation::Experiment(experiment_eval)
    }
}

impl From<DynamicConfigEvaluation> for AnyEvaluation {
    fn from(dynamic_config_evalation: DynamicConfigEvaluation) -> Self {
        AnyEvaluation::DynamicConfig(dynamic_config_evalation)
    }
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
    pub value: HashMap<String, Value>,

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
    pub value: HashMap<String, Value>,

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

    pub value: HashMap<String, Value>,

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
