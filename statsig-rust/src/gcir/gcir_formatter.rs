use crate::gcir::feature_gates_processor::get_gate_evaluations;

use crate::interned_string::InternedString;
use crate::specs_response::spec_types::SessionReplayTrigger;
use crate::{
    evaluation::evaluator::{Evaluator, SpecType},
    evaluation::evaluator_context::EvaluatorContext,
    initialize_evaluations_response::InitializeEvaluationsResponse,
    initialize_response::InitializeResponse,
    statsig_metadata::StatsigMetadata,
    StatsigErr,
};

use crate::StatsigUser;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;

use super::dynamic_configs_processor::{
    get_dynamic_config_evaluations, get_dynamic_config_evaluations_v2,
};
use super::feature_gates_processor::get_gate_evaluations_v2;
use super::gcir_options::ClientInitResponseOptions;
use super::layer_configs_processor::{get_layer_evaluations, get_layer_evaluations_v2};
use super::param_stores_processor::get_serializeable_param_stores;

#[derive(Deserialize)]
pub enum GCIRResponseFormat {
    Initialize,                             // v1
    InitializeWithSecondaryExposureMapping, // v2
}

impl GCIRResponseFormat {
    #[must_use]
    pub fn from_string(input: &str) -> Option<Self> {
        match input {
            "v1" => Some(GCIRResponseFormat::Initialize),
            "v2" => Some(GCIRResponseFormat::InitializeWithSecondaryExposureMapping),
            _ => None,
        }
    }
}

pub struct GCIRFormatter;

impl GCIRFormatter {
    pub fn generate_v1_format(
        context: &mut EvaluatorContext,
        options: &ClientInitResponseOptions,
    ) -> Result<InitializeResponse, StatsigErr> {
        let mut sec_expo_hash_memo = HashMap::new();

        let gates = get_gate_evaluations(context, options, &mut sec_expo_hash_memo)?;
        let configs = get_dynamic_config_evaluations(context, options, &mut sec_expo_hash_memo)?;
        let layers = get_layer_evaluations(context, options, &mut sec_expo_hash_memo)?;

        let param_stores = get_serializeable_param_stores(context, options);
        let evaluated_keys = get_evaluated_keys(context.user.user_ref);
        let session_replay_info = get_session_replay_info(context, options);

        Ok(InitializeResponse {
            feature_gates: gates,
            dynamic_configs: configs,
            layer_configs: layers,
            time: context.specs_data.time,
            has_updates: true,
            hash_used: options.get_hash_algorithm().to_string(),
            user: context.user.to_loggable(),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: get_sdk_info(),
            param_stores,
            can_record_session: session_replay_info.can_record_session,
            session_recording_rate: session_replay_info.session_recording_rate,
            recording_blocked: session_replay_info.recording_blocked,
            passes_session_recording_targeting: session_replay_info
                .passes_session_recording_targeting,
            session_recording_event_triggers: session_replay_info.session_recording_event_triggers,
            session_recording_exposure_triggers: session_replay_info
                .session_recording_exposure_triggers,
            pa_hash: context.user.get_hashed_private_attributes(),
        })
    }

    pub fn generate_v2_format(
        context: &mut EvaluatorContext,
        options: &ClientInitResponseOptions,
    ) -> Result<InitializeEvaluationsResponse, StatsigErr> {
        let mut sec_expo_hash_memo = HashMap::new();
        let mut exposures = HashMap::new();

        let param_stores = get_serializeable_param_stores(context, options);
        let evaluated_keys = get_evaluated_keys(context.user.user_ref);
        let session_replay_info = get_session_replay_info(context, options);

        Ok(InitializeEvaluationsResponse {
            feature_gates: get_gate_evaluations_v2(
                context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            dynamic_configs: get_dynamic_config_evaluations_v2(
                context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            layer_configs: get_layer_evaluations_v2(
                context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            time: context.specs_data.time,
            has_updates: true,
            hash_used: options.get_hash_algorithm().to_string(),
            user: context.user.to_loggable(),
            pa_hash: context.user.get_hashed_private_attributes(),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: get_sdk_info(),
            param_stores,
            exposures,
            can_record_session: session_replay_info.can_record_session,
            session_recording_rate: session_replay_info.session_recording_rate,
            recording_blocked: session_replay_info.recording_blocked,
            passes_session_recording_targeting: session_replay_info
                .passes_session_recording_targeting,
            session_recording_event_triggers: session_replay_info.session_recording_event_triggers,
            session_recording_exposure_triggers: session_replay_info
                .session_recording_exposure_triggers,
        })
    }
}

fn get_evaluated_keys(user: &StatsigUser) -> HashMap<InternedString, InternedString> {
    let mut evaluated_keys = HashMap::new();

    if let Some(user_id) = user.data.user_id.as_ref() {
        evaluated_keys.insert(
            InternedString::from_str_ref("userID"),
            user_id
                .string_value
                .as_ref()
                .map(|s| s.value.clone())
                .unwrap_or_default(),
        );
    }

    if let Some(custom_ids) = user.data.custom_ids.as_ref() {
        for (key, value) in custom_ids {
            evaluated_keys.insert(
                InternedString::from_str_ref(key.as_str()),
                value
                    .string_value
                    .as_ref()
                    .map(|s| s.value.clone())
                    .unwrap_or_default(),
            );
        }
    }

    evaluated_keys
}

fn get_sdk_info() -> HashMap<String, String> {
    let metadata = StatsigMetadata::get_metadata();
    HashMap::from([
        ("sdkType".to_string(), metadata.sdk_type),
        ("sdkVersion".to_string(), metadata.sdk_version),
        ("sessionId".to_string(), metadata.session_id),
    ])
}

pub struct GCIRSessionReplayInfo {
    pub can_record_session: Option<bool>,
    pub session_recording_rate: Option<f64>,
    pub recording_blocked: Option<bool>,
    pub passes_session_recording_targeting: Option<bool>,
    pub session_recording_event_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_exposure_triggers: Option<HashMap<String, SessionReplayTrigger>>,
}

fn get_session_replay_info(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
) -> GCIRSessionReplayInfo {
    let mut session_replay_info = GCIRSessionReplayInfo {
        can_record_session: None,
        session_recording_rate: None,
        recording_blocked: None,
        passes_session_recording_targeting: None,
        session_recording_event_triggers: None,
        session_recording_exposure_triggers: None,
    };

    let session_replay_data = match &context.specs_data.session_replay_info {
        Some(data) => data,
        None => return session_replay_info,
    };

    session_replay_info.can_record_session = Some(true);
    session_replay_info.recording_blocked = session_replay_data.recording_blocked;
    if session_replay_data.recording_blocked == Some(true) {
        session_replay_info.can_record_session = Some(false);
    }

    let targeting_gate_name = &session_replay_data.targeting_gate;

    if let Some(gate_name) = targeting_gate_name {
        match Evaluator::evaluate(context, gate_name.clone().as_str(), &SpecType::Gate) {
            Ok(_result) => {
                session_replay_info.passes_session_recording_targeting =
                    Some(context.result.bool_value);
                if !context.result.bool_value {
                    session_replay_info.can_record_session = Some(false);
                }
            }
            Err(_e) => {
                session_replay_info.passes_session_recording_targeting = Some(false);
                session_replay_info.can_record_session = Some(false);
            }
        }
    }

    let mut rng = rand::thread_rng();
    let random: f64 = rng.gen::<f64>();

    if let Some(rate) = &session_replay_data.sampling_rate {
        session_replay_info.session_recording_rate = Some(*rate);
        if random > *rate {
            session_replay_info.can_record_session = Some(false);
        }
    }

    if let Some(triggers) = &session_replay_data.session_recording_event_triggers {
        let mut new_event_triggers = HashMap::new();
        for (key, trigger) in triggers {
            let mut new_trigger = SessionReplayTrigger {
                values: trigger.values.clone(),
                sampling_rate: None,
                passes_sampling: None,
            };
            if let Some(rate) = &trigger.sampling_rate {
                new_trigger.passes_sampling = Some(random <= *rate);
            }
            new_event_triggers.insert(key.clone(), new_trigger);
        }
        session_replay_info.session_recording_event_triggers = Some(new_event_triggers);
    }

    if let Some(triggers) = &session_replay_data.session_recording_exposure_triggers {
        let mut new_exposure_triggers = HashMap::new();
        for (key, trigger) in triggers {
            let mut new_trigger = SessionReplayTrigger {
                values: trigger.values.clone(),
                sampling_rate: None,
                passes_sampling: None,
            };
            if let Some(rate) = &trigger.sampling_rate {
                new_trigger.passes_sampling = Some(random <= *rate);
            }
            new_exposure_triggers.insert(
                context
                    .hashing
                    .hash(key.as_str(), options.get_hash_algorithm()),
                new_trigger,
            );
        }
        session_replay_info.session_recording_exposure_triggers = Some(new_exposure_triggers);
    }

    session_replay_info
}
