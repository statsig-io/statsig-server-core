use crate::{
    evaluation::evaluation_types::{AnyConfigEvaluation, GateEvaluation, LayerEvaluation},
    specs_response::param_store_types::Parameter,
    user::{StatsigUserInternal, StatsigUserLoggable},
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct InitializeResponse {
    pub feature_gates: HashMap<String, GateEvaluation>,
    pub dynamic_configs: HashMap<String, AnyConfigEvaluation>,
    pub layer_configs: HashMap<String, LayerEvaluation>,
    pub time: u64,
    pub has_updates: bool,
    pub hash_used: String,
    pub user: StatsigUserLoggable,
    #[serde(rename = "sdkInfo")]
    pub sdk_info: HashMap<String, String>,
    #[serde(rename = "sdkParams")]
    pub sdk_params: HashMap<String, String>,
    pub evaluated_keys: HashMap<String, String>,
    pub param_stores: HashMap<String, HashMap<String, Parameter>>,
}

impl InitializeResponse {
    pub fn blank(user: StatsigUserInternal) -> Self {
        Self {
            feature_gates: Default::default(),
            dynamic_configs: Default::default(),
            layer_configs: Default::default(),
            time: 0,
            has_updates: false,
            hash_used: Default::default(),
            user: user.to_loggable(),
            sdk_params: Default::default(),
            evaluated_keys: Default::default(),
            sdk_info: Default::default(),
            param_stores: Default::default(),
        }
    }
}
