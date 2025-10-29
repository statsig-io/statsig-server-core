use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluation_types_initialize_v2::{
    AnyConfigEvaluationInitV2, GateEvaluationInitV2, LayerEvaluationInitV2,
};
use crate::interned_string::InternedString;
use crate::specs_response::param_store_types::Parameter;
use crate::specs_response::spec_types::SessionReplayTrigger;
use crate::user::{StatsigUserInternal, StatsigUserLoggable};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct InitializeV2Response {
    pub feature_gates: HashMap<String, GateEvaluationInitV2>,
    pub dynamic_configs: HashMap<String, AnyConfigEvaluationInitV2>,
    pub layer_configs: HashMap<String, LayerEvaluationInitV2>,
    pub param_stores: HashMap<String, HashMap<String, Parameter>>,
    pub time: u64,
    pub has_updates: bool,
    pub hash_used: String,
    pub user: StatsigUserLoggable,
    #[serde(rename = "sdkInfo")]
    pub sdk_info: HashMap<String, String>,
    #[serde(rename = "sdkParams")]
    pub sdk_params: HashMap<String, String>,
    pub evaluated_keys: HashMap<InternedString, InternedString>,
    pub exposures: HashMap<InternedString, SecondaryExposure>,
    pub can_record_session: Option<bool>,
    pub session_recording_rate: Option<f64>,
    pub recording_blocked: Option<bool>,
    pub passes_session_recording_targeting: Option<bool>,
    pub session_recording_event_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_exposure_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub pa_hash: Option<String>,
    pub values: HashMap<InternedString, DynamicReturnable>,
    pub response_format: String,
}

impl InitializeV2Response {
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
            can_record_session: Default::default(),
            session_recording_rate: Default::default(),
            recording_blocked: Default::default(),
            passes_session_recording_targeting: Default::default(),
            session_recording_event_triggers: Default::default(),
            session_recording_exposure_triggers: Default::default(),
            pa_hash: Default::default(),
            response_format: "init-v2".to_string(),
            values: Default::default(),
        }
    }
}
