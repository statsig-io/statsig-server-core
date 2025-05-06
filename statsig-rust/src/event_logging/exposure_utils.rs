use super::exposer_sampling::{EvtSamplingDecision, EvtSamplingMode};
use crate::evaluation::evaluation_details::EvaluationDetails;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) fn get_statsig_metadata_with_sampling_decision(
    sampling_decision: EvtSamplingDecision,
) -> HashMap<String, Value> {
    let (sampling_rate, mode, was_sampled) = match sampling_decision {
        EvtSamplingDecision::Sampled(sampling_rate, mode, was_sampled) => {
            (sampling_rate, mode, was_sampled)
        }
        _ => return HashMap::new(),
    };

    let mut statsig_metadata: HashMap<String, Value> = HashMap::new();
    statsig_metadata.insert("samplingRate".into(), sampling_rate.into());

    let mode = match mode {
        EvtSamplingMode::On => "on",
        EvtSamplingMode::Shadow => "shadow",
    };
    statsig_metadata.insert("samplingMode".into(), mode.into());

    // weird naming, but in 'shadow' mode, we log events that would have been dropped
    let shadow_logged = match was_sampled {
        true => "logged",
        false => "dropped",
    };
    statsig_metadata.insert("shadowLogged".into(), shadow_logged.into());

    statsig_metadata
}

pub(crate) fn get_metadata_with_details(
    evaluation_details: EvaluationDetails,
) -> HashMap<String, String> {
    let mut metadata: HashMap<String, String> = HashMap::new();

    metadata.insert("reason".into(), evaluation_details.reason);

    if let Some(lcut) = evaluation_details.lcut {
        metadata.insert("lcut".into(), lcut.to_string());
    }

    if let Some(received_at) = evaluation_details.received_at {
        metadata.insert("receivedAt".into(), received_at.to_string());
    }

    metadata
}
