use crate::{
    evaluation::evaluation_types::ExposureSamplingInfo,
    event_logging::{
        event_logger::ExposureTrigger,
        exposer_sampling::EvtSamplingDecision,
        exposure_utils::{get_metadata_with_details, get_statsig_metadata_with_sampling_decision},
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, GATE_EXPOSURE_EVENT_NAME},
    },
    statsig_types::FeatureGate,
    user::{StatsigUserInternal, StatsigUserLoggable},
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

pub struct EnqueueGateExpoOp<'a> {
    pub user: &'a StatsigUserInternal<'a, 'a>,
    pub gate: &'a FeatureGate,
    pub trigger: ExposureTrigger,
}

impl EnqueueOperation for EnqueueGateExpoOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let evaluation = self.gate.__evaluation.as_ref();
        let secondary_exposures = evaluation.map(|eval| &eval.base.secondary_exposures);

        QueuedEvent::GateExposure(QueuedGateExposureEvent {
            user: self.user.to_loggable(),
            gate_name: self.gate.name.clone(),
            value: self.gate.value,
            rule_id: self.gate.rule_id.clone(),
            secondary_exposures: secondary_exposures.cloned(),
            evaluation_details: self.gate.details.clone(),
            version: self.gate.__version,
            exposure_trigger: self.trigger,
            sampling_decision,
            override_config_name: self.gate.__override_config_name.clone(),
        })
    }
}

impl<'a> QueuedExposure<'a> for EnqueueGateExpoOp<'a> {
    fn create_exposure_sampling_key(&self) -> String {
        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&self.gate.name);
        sampling_key.push_str(";u:");
        sampling_key.push_str(&self.user.create_sampling_key());
        sampling_key.push_str(";r:");
        sampling_key.push_str(&self.gate.rule_id);
        sampling_key.push_str(";v:");
        sampling_key.push_str(if self.gate.value { "true" } else { "false" });
        sampling_key
    }

    fn create_spec_sampling_key(&self) -> String {
        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&self.gate.name);
        sampling_key.push_str(";r:");
        sampling_key.push_str(&self.gate.rule_id);
        sampling_key
    }

    fn get_rule_id_ref(&self) -> &'a str {
        &self.gate.rule_id
    }

    fn get_sampling_info_ref(&self) -> Option<&'a ExposureSamplingInfo> {
        self.gate.__evaluation.as_ref()?.base.sampling_info.as_ref()
    }

    fn get_sampling_rate(&self) -> Option<u64> {
        self.gate
            .__evaluation
            .as_ref()?
            .base
            .sampling_info
            .as_ref()?
            .sampling_rate
    }
}

pub struct QueuedGateExposureEvent {
    pub user: StatsigUserLoggable,
    pub gate_name: String,
    pub value: bool,
    pub rule_id: String,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub evaluation_details: EvaluationDetails,
    pub version: Option<u32>,
    pub exposure_trigger: ExposureTrigger,
    pub sampling_decision: EvtSamplingDecision,
    pub override_config_name: Option<String>,
}

impl QueuedGateExposureEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("gate".into(), self.gate_name);
        metadata.insert("gateValue".into(), self.value.to_string());
        metadata.insert("ruleID".into(), self.rule_id);

        if self.exposure_trigger == ExposureTrigger::Manual {
            metadata.insert("isManualExposure".into(), "true".into());
        }

        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert("overrideConfigName".into(), override_config_name);
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_decision(self.sampling_decision);

        let event = StatsigEvent {
            event_name: GATE_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(self.user, event, self.secondary_exposures)
    }
}
