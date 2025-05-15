use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluation_types_v2::{
    AnyConfigEvaluationV2, GateEvaluationV2, LayerEvaluationV2,
};
use crate::specs_response::param_store_types::Parameter;
use crate::user::{StatsigUserInternal, StatsigUserLoggable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct InitializeEvaluationsResponse {
    pub feature_gates: HashMap<String, GateEvaluationV2>,
    pub dynamic_configs: HashMap<String, AnyConfigEvaluationV2>,
    pub layer_configs: HashMap<String, LayerEvaluationV2>,
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
    pub exposures: HashMap<String, SecondaryExposure>,
}

impl InitializeEvaluationsResponse {
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
            exposures: Default::default(),
        }
    }
}
