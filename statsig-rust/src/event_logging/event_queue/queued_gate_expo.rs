use std::borrow::Cow;

use crate::{
    evaluation::evaluation_types::{ExtraExposureInfo, GateEvaluation},
    event_logging::{
        event_logger::ExposureTrigger,
        exposable_string::ExposableString,
        exposer_sampling::EvtSamplingDecision,
        exposure_utils::{get_metadata_with_details, get_statsig_metadata_with_sampling_decision},
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, GATE_EXPOSURE_EVENT_NAME},
    },
    user::{StatsigUserInternal, StatsigUserLoggable},
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

pub struct EnqueueGateExpoOp<'a> {
    pub user: &'a StatsigUserInternal<'a, 'a>,
    pub evaluation: Option<Cow<'a, GateEvaluation>>,
    pub details: EvaluationDetails,
    pub trigger: ExposureTrigger,
}

impl EnqueueOperation for EnqueueGateExpoOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let (
            gate_name, //
            rule_id,
            value,
            version,
            override_config_name,
            secondary_exposures,
        ) = extract_from_cow(self.evaluation);

        QueuedEvent::GateExposure(QueuedGateExposureEvent {
            user: self.user.to_loggable(),
            gate_name,
            value,
            version,
            rule_id,
            secondary_exposures,
            evaluation_details: self.details,
            override_config_name,
            exposure_trigger: self.trigger,
            sampling_decision,
        })
    }
}

impl<'a> QueuedExposure<'a> for EnqueueGateExpoOp<'a> {
    fn create_exposure_sampling_key(&self) -> String {
        let gate_name = gate_name_ref(&self.evaluation);
        let rule_id = rule_id_ref(&self.evaluation);

        let mut sampling_key = String::from("n:");
        sampling_key.push_str(gate_name);
        sampling_key.push_str(";u:");
        sampling_key.push_str(&self.user.create_sampling_key());
        sampling_key.push_str(";r:");
        sampling_key.push_str(rule_id);
        sampling_key.push_str(";v:");
        sampling_key.push_str(value_as_str(&self.evaluation));
        sampling_key
    }

    fn create_spec_sampling_key(&self) -> String {
        let gate_name = gate_name_ref(&self.evaluation);
        let rule_id = rule_id_ref(&self.evaluation);

        let mut sampling_key = String::from("n:");
        sampling_key.push_str(gate_name);
        sampling_key.push_str(";r:");
        sampling_key.push_str(rule_id);
        sampling_key
    }

    fn get_rule_id_ref(&'a self) -> &'a str {
        rule_id_ref(&self.evaluation)
    }

    fn get_extra_exposure_info_ref(&'a self) -> Option<&'a ExtraExposureInfo> {
        self.evaluation.as_ref()?.base.exposure_info.as_ref()
    }
}

pub struct QueuedGateExposureEvent {
    pub user: StatsigUserLoggable,
    pub gate_name: String,
    pub value: bool,
    pub rule_id: ExposableString,
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

        // This mostly occurs in the EventLogger bg thread, so it's ok to use unperformant_to_string
        // todo: investigate how to avoid cloning the inner value entirely
        metadata.insert("ruleID".into(), self.rule_id.unperformant_to_string());

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

type ExtractInfoResult = (
    String,
    ExposableString,
    bool,
    Option<u32>,
    Option<String>,
    Option<Vec<SecondaryExposure>>,
);

fn extract_from_cow(moo: Option<Cow<'_, GateEvaluation>>) -> ExtractInfoResult {
    let moo = match moo {
        Some(m) => m,
        None => {
            return (
                String::new(),
                ExposableString::default(),
                false,
                None,
                None,
                None,
            )
        }
    };

    match moo {
        Cow::Borrowed(evaluation) => {
            let name = evaluation.base.name.clone();
            let rule_id = evaluation.base.rule_id.clone();
            let value = evaluation.value;
            let expo_info = evaluation.base.exposure_info.clone();
            let secondary_exposures = evaluation.base.secondary_exposures.clone();

            let version = expo_info.as_ref().and_then(|info| info.version);
            let override_config_name = expo_info
                .as_ref()
                .and_then(|info| info.override_config_name.clone());

            (
                name,
                rule_id,
                value,
                version,
                override_config_name,
                Some(secondary_exposures),
            )
        }
        Cow::Owned(evaluation) => {
            let name = evaluation.base.name;
            let rule_id = evaluation.base.rule_id;
            let value = evaluation.value;
            let expo_info = evaluation.base.exposure_info;
            let secondary_exposures = evaluation.base.secondary_exposures;

            let version = expo_info.as_ref().and_then(|info| info.version);
            let override_config_name = expo_info.and_then(|info| info.override_config_name.clone());

            (
                name,
                rule_id,
                value,
                version,
                override_config_name,
                Some(secondary_exposures),
            )
        }
    }
}

fn gate_name_ref<'a>(moo: &'a Option<Cow<'a, GateEvaluation>>) -> &'a str {
    moo.as_ref().map_or("", |x| x.base.name.as_str())
}

fn rule_id_ref<'a>(moo: &'a Option<Cow<'a, GateEvaluation>>) -> &'a str {
    moo.as_ref().map_or("", |x| x.base.rule_id.as_str())
}

fn value_as_str<'a>(moo: &'a Option<Cow<'a, GateEvaluation>>) -> &'a str {
    if moo.as_ref().is_some_and(|x| x.value) {
        "T"
    } else {
        "F"
    }
}
