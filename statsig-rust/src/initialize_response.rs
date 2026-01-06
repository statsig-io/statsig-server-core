use crate::{
    evaluation::evaluation_types::{AnyConfigEvaluation, GateEvaluation, LayerEvaluation},
    gcir::gcir_formatter::EvaluatedKeys,
    specs_response::{
        param_store_types::Parameter,
        spec_types::{SessionReplayPrivacySetting, SessionReplayTrigger},
    },
    user::{StatsigUserInternal, StatsigUserLoggable},
};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[skip_serializing_none]
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
    pub evaluated_keys: EvaluatedKeys,
    pub param_stores: HashMap<String, HashMap<String, Parameter>>,
    pub can_record_session: Option<bool>,
    pub session_recording_rate: Option<f64>,
    pub recording_blocked: Option<bool>,
    pub passes_session_recording_targeting: Option<bool>,
    pub session_recording_event_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_exposure_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_privacy_settings: Option<SessionReplayPrivacySetting>,
    pub pa_hash: Option<String>,
    pub full_checksum: Option<String>,
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
            evaluated_keys: EvaluatedKeys::default(),
            sdk_info: Default::default(),
            param_stores: Default::default(),
            can_record_session: Default::default(),
            session_recording_rate: Default::default(),
            recording_blocked: Default::default(),
            passes_session_recording_targeting: Default::default(),
            session_recording_event_triggers: Default::default(),
            session_recording_exposure_triggers: Default::default(),
            session_recording_privacy_settings: Default::default(),
            pa_hash: user.get_hashed_private_attributes(),
            full_checksum: Default::default(),
        }
    }

    pub fn blank_without_user() -> Self {
        Self {
            feature_gates: Default::default(),
            dynamic_configs: Default::default(),
            layer_configs: Default::default(),
            time: 0,
            has_updates: false,
            hash_used: Default::default(),
            user: StatsigUserLoggable::default(),
            sdk_params: Default::default(),
            evaluated_keys: Default::default(),
            sdk_info: Default::default(),
            param_stores: Default::default(),
            can_record_session: Default::default(),
            session_recording_rate: Default::default(),
            recording_blocked: Default::default(),
            passes_session_recording_targeting: Default::default(),
            session_recording_event_triggers: Default::default(),
            session_recording_exposure_triggers: Default::default(),
            session_recording_privacy_settings: Default::default(),
            pa_hash: None,
            full_checksum: Default::default(),
        }
    }
}
