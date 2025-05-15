use crate::{
    evaluation::evaluation_types::ExtraExposureInfo,
    event_logging::{
        event_logger::ExposureTrigger,
        exposure_sampling::EvtSamplingDecision,
        exposure_utils::{get_metadata_with_details, get_statsig_metadata_with_sampling_decision},
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, CONFIG_EXPOSURE_EVENT_NAME},
    },
    statsig_types::Experiment,
    user::{StatsigUserInternal, StatsigUserLoggable},
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

pub struct EnqueueExperimentExpoOp<'a> {
    pub user: &'a StatsigUserInternal<'a, 'a>,
    pub experiment: &'a Experiment,
    pub trigger: ExposureTrigger,
}

impl EnqueueOperation for EnqueueExperimentExpoOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let evaluation = self.experiment.__evaluation.as_ref();
        let secondary_exposures = evaluation.map(|eval| &eval.base.secondary_exposures);
        let exposure_info = evaluation.and_then(|eval| eval.base.exposure_info.as_ref());
        let (version, override_config_name) = exposure_info
            .map(|info| (info.version, info.override_config_name.clone()))
            .unwrap_or_default();

        QueuedEvent::ExperimentExposure(QueuedExperimentExposureEvent {
            user: self.user.to_loggable(),
            experiment_name: self.experiment.name.clone(),
            rule_id: self.experiment.rule_id.clone(),
            secondary_exposures: secondary_exposures.cloned(),
            evaluation_details: self.experiment.details.clone(),
            version,
            exposure_trigger: self.trigger,
            sampling_decision,
            override_config_name,
        })
    }
}

impl<'a> QueuedExposure<'a> for EnqueueExperimentExpoOp<'a> {
    fn create_exposure_sampling_key(&self) -> String {
        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&self.experiment.name);
        sampling_key.push_str(";u:");
        sampling_key.push_str(&self.user.user_ref.data.create_hash().to_string());
        sampling_key.push_str(&self.experiment.rule_id);
        sampling_key
    }

    fn create_spec_sampling_key(&self) -> String {
        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&self.experiment.name);
        sampling_key.push_str(";r:");
        sampling_key.push_str(&self.experiment.rule_id);
        sampling_key
    }

    fn get_rule_id_ref(&self) -> &'a str {
        &self.experiment.rule_id
    }

    fn get_extra_exposure_info_ref(&self) -> Option<&'a ExtraExposureInfo> {
        self.experiment
            .__evaluation
            .as_ref()?
            .base
            .exposure_info
            .as_ref()
    }
}

pub struct QueuedExperimentExposureEvent {
    pub user: StatsigUserLoggable,
    pub experiment_name: String,
    pub rule_id: String,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub evaluation_details: EvaluationDetails,
    pub version: Option<u32>,
    pub exposure_trigger: ExposureTrigger,
    pub sampling_decision: EvtSamplingDecision,
    pub override_config_name: Option<String>,
}

impl QueuedExperimentExposureEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.experiment_name);
        metadata.insert("ruleID".into(), self.rule_id);

        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }

        if self.exposure_trigger == ExposureTrigger::Manual {
            metadata.insert("isManualExposure".into(), "true".into());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert("overrideConfigName".into(), override_config_name);
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_decision(self.sampling_decision);

        let event = StatsigEvent {
            event_name: CONFIG_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(
            self.user,
            event,
            Some(self.secondary_exposures.unwrap_or_default()),
        )
    }
}
