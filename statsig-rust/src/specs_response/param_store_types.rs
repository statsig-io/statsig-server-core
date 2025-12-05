use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::interned_string::InternedString;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum Parameter {
    StaticValue(StaticValueParameter),
    Gate(GateParameter),
    DynamicConfig(DynamicConfigParameter),
    Experiment(ExperimentParameter),
    Layer(LayerParameter),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ParameterStore {
    pub parameters: HashMap<String, Parameter>,
    #[serde(rename = "targetAppIDs")]
    pub target_app_ids: Option<Vec<InternedString>>,
    pub version: Option<u32>,
    pub checksum: Option<InternedString>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StaticValueParameter {
    pub ref_type: String,
    pub param_type: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GateParameter {
    pub ref_type: String,
    pub param_type: String,
    pub gate_name: String,
    pub pass_value: Value,
    pub fail_value: Value,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DynamicConfigParameter {
    pub ref_type: String,
    pub param_type: String,
    pub config_name: String,
    pub param_name: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExperimentParameter {
    pub ref_type: String,
    pub param_type: String,
    pub experiment_name: String,
    pub param_name: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct LayerParameter {
    pub ref_type: String,
    pub param_type: String,
    pub layer_name: String,
    pub param_name: String,
}
