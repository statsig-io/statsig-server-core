use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::dynamic_string::DynamicString;
use crate::evaluation::dynamic_value::DynamicValue;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    #[serde(rename = "type")]
    pub _type: String,
    pub salt: String,
    pub default_value: DynamicReturnable,
    pub enabled: bool,
    pub rules: Vec<Rule>,
    pub id_type: String,
    pub explicit_parameters: Option<Vec<String>>,
    pub entity: String,
    pub has_shared_params: Option<bool>,
    pub is_active: Option<bool>,
    pub version: Option<u32>,
    #[serde(rename = "targetAppIDs")]
    pub target_app_ids: Option<Vec<String>>,
    pub forward_all_exposures: Option<bool>,
    pub fields_used: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub name: String,
    pub pass_percentage: f64,
    pub return_value: DynamicReturnable,
    pub id: String,
    pub salt: Option<String>,
    pub conditions: Vec<String>,
    pub id_type: DynamicString,
    pub group_name: Option<String>,
    pub config_delegate: Option<String>,
    pub is_experiment_group: Option<bool>,
    pub sampling_rate: Option<u64>,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub target_value: Option<DynamicValue>,
    pub operator: Option<String>,
    pub field: Option<DynamicString>,
    pub additional_values: Option<HashMap<String, DynamicValue>>,
    pub id_type: DynamicString,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OverrideRule {
    pub rule_name: String,
    pub start_time: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ConfigMapping {
    pub new_config_name: String,
    pub rules: Vec<OverrideRule>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SpecsResponseFull {
    pub feature_gates: HashMap<String, Spec>,
    pub dynamic_configs: HashMap<String, Spec>,
    pub layer_configs: HashMap<String, Spec>,
    pub condition_map: HashMap<String, Condition>,
    pub experiment_to_layer: HashMap<String, String>,
    pub has_updates: bool,
    pub time: u64,
    pub checksum: Option<String>,
    pub default_environment: Option<String>,
    pub app_id: Option<DynamicValue>,
    pub sdk_keys_to_app_ids: Option<HashMap<String, DynamicValue>>,
    pub hashed_sdk_keys_to_app_ids: Option<HashMap<String, DynamicValue>>,
    pub diagnostics: Option<HashMap<String, f64>>,
    pub param_stores: Option<HashMap<String, ParameterStore>>,
    pub sdk_configs: Option<HashMap<String, DynamicValue>>,
    pub cmab_configs: Option<HashMap<String, CMABConfig>>,
    pub overrides: Option<HashMap<String, Vec<ConfigMapping>>>,
    pub override_rules: Option<HashMap<String, Rule>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CMABConfig {
    pub salt: String,
    #[serde(rename = "targetAppIDs")]
    pub target_app_ids: Option<Vec<String>>,
    pub default_value: DynamicReturnable,
    pub id_type: DynamicString,
    pub enabled: bool,
    pub version: u32,
    pub sample_rate: f64,
    pub higher_is_better: bool,
    pub groups: Vec<CMABGroup>,
    pub config: Option<HashMap<String, CMABGroupConfig>>,
    pub targeting_gate_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CMABGroup {
    pub name: String,
    pub parameter_values: DynamicReturnable,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CMABGroupConfig {
    pub alpha: f64,
    pub intercept: f64,
    pub records: u64,
    pub weights_numerical: HashMap<String, f64>,
    pub weights_categorical: HashMap<String, HashMap<String, f64>>,
}

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
    pub target_app_ids: Option<Vec<String>>,
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

#[derive(Deserialize)]
pub struct SpecsResponseNoUpdates {
    pub has_updates: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SpecsResponse {
    Full(Box<SpecsResponseFull>),
    NoUpdates(SpecsResponseNoUpdates),
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ConditionInternal {
            #[serde(rename = "type")]
            condition_type: String,
            target_value: Option<DynamicValue>,
            operator: Option<String>,
            field: Option<DynamicString>,
            additional_values: Option<HashMap<String, DynamicValue>>,
            id_type: DynamicString,
        }

        let mut internal = ConditionInternal::deserialize(deserializer)?;

        if let Some(ref op) = internal.operator {
            if op == "str_matches" {
                if let Some(ref mut tv) = internal.target_value {
                    tv.compile_regex();
                }
            }
        }

        Ok(Condition {
            condition_type: internal.condition_type,
            target_value: internal.target_value,
            operator: internal.operator,
            field: internal.field,
            additional_values: internal.additional_values,
            id_type: internal.id_type,
        })
    }
}
