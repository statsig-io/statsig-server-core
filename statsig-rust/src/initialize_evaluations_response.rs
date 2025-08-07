use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluation_types_v2::{
    AnyConfigEvaluationV2, GateEvaluationV2, LayerEvaluationV2,
};
use crate::specs_response::param_store_types::Parameter;
use crate::specs_response::spec_types::SessionReplayTrigger;
use crate::user::{StatsigUserInternal, StatsigUserLoggable};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[skip_serializing_none]
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
    pub can_record_session: Option<bool>,
    pub session_recording_rate: Option<f64>,
    pub recording_blocked: Option<bool>,
    pub passes_session_recording_targeting: Option<bool>,
    pub session_recording_event_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_exposure_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub pa_hash: Option<String>,
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
            can_record_session: Default::default(),
            session_recording_rate: Default::default(),
            recording_blocked: Default::default(),
            passes_session_recording_targeting: Default::default(),
            session_recording_event_triggers: Default::default(),
            session_recording_exposure_triggers: Default::default(),
            pa_hash: Default::default(),
        }
    }
}
