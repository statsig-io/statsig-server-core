use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    evaluation::{dynamic_returnable::DynamicReturnable, dynamic_string::DynamicString},
    event_logging::exposable_string::ExposableString,
    interned_string::InternedString,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CMABConfig {
    pub salt: String,
    #[serde(rename = "targetAppIDs")]
    pub target_app_ids: Option<Vec<InternedString>>,
    pub default_value: DynamicReturnable,
    pub id_type: DynamicString,
    pub enabled: bool,
    pub version: u32,
    pub sample_rate: f64,
    pub higher_is_better: bool,
    pub groups: Vec<CMABGroup>,
    pub config: Option<HashMap<String, CMABGroupConfig>>,
    pub targeting_gate_name: Option<InternedString>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CMABGroup {
    pub name: String,
    pub parameter_values: DynamicReturnable,
    pub id: ExposableString,
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
