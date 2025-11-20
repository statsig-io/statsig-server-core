use crate::{
    evaluation::evaluation_types::ExtraExposureInfo,
    event_logging::{
        event_logger::ExposureTrigger,
        exposure_sampling::{EvtSamplingDecision, ExposureSamplingKey},
        exposure_utils::{get_metadata_with_details, get_statsig_metadata_with_sampling_decision},
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, CONFIG_EXPOSURE_EVENT_NAME},
    },
    interned_string::InternedString,
    statsig_types::DynamicConfig,
    user::{StatsigUserInternal, StatsigUserLoggable},
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

pub struct EnqueueConfigExpoOp<'a> {
    pub user: &'a StatsigUserInternal<'a, 'a>,
    pub config: &'a DynamicConfig,
    pub trigger: ExposureTrigger,
    pub exposure_time: u64,
}

impl EnqueueOperation for EnqueueConfigExpoOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure<'_>> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let evaluation = self.config.__evaluation.as_ref();
        let secondary_exposures = evaluation.map(|eval| &eval.base.secondary_exposures);
        let exposure_info = evaluation.and_then(|eval| eval.base.exposure_info.as_ref());
        let (version, override_config_name) = exposure_info
            .map(|info| (info.version, info.override_config_name.clone()))
            .unwrap_or_default();

        QueuedEvent::ConfigExposure(QueuedConfigExposureEvent {
            user: self.user.to_loggable(),
            config_name: self.config.name.clone(),
            rule_id: self.config.rule_id.clone(),
            rule_passed: evaluation.map(|eval| eval.passed),
            secondary_exposures: secondary_exposures.cloned(),
            evaluation_details: self.config.details.clone(),
            version,
            exposure_trigger: self.trigger,
            sampling_decision,
            override_config_name,
            exposure_time: self.exposure_time,
        })
    }
}

impl<'a> QueuedExposure<'a> for EnqueueConfigExpoOp<'a> {
    fn create_exposure_sampling_key(&self) -> ExposureSamplingKey {
        let user_data = &self.user.user_ref.data;
        let evaluation = self.config.__evaluation.as_ref().map(|e| &e.base);
        let passed = self.config.__evaluation.as_ref().is_some_and(|e| e.passed);

        ExposureSamplingKey::new(evaluation, user_data.as_ref(), passed as u64)
    }

    fn get_rule_id_ref(&self) -> &'a str {
        &self.config.rule_id
    }

    fn get_extra_exposure_info_ref(&self) -> Option<&'a ExtraExposureInfo> {
        self.config
            .__evaluation
            .as_ref()?
            .base
            .exposure_info
            .as_ref()
    }
}

pub struct QueuedConfigExposureEvent {
    pub user: StatsigUserLoggable,
    pub config_name: String,
    pub rule_id: String,
    pub rule_passed: Option<bool>,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub evaluation_details: EvaluationDetails,
    pub version: Option<u32>,
    pub exposure_trigger: ExposureTrigger,
    pub sampling_decision: EvtSamplingDecision,
    pub override_config_name: Option<InternedString>,
    pub exposure_time: u64,
}

impl QueuedConfigExposureEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.config_name);
        metadata.insert("ruleID".into(), self.rule_id);

        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }

        if let Some(rule_passed) = self.rule_passed {
            metadata.insert("rulePassed".into(), rule_passed.to_string());
        }

        if self.exposure_trigger == ExposureTrigger::Manual {
            metadata.insert("isManualExposure".into(), "true".into());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert(
                "overrideConfigName".into(),
                override_config_name.unperformant_to_string(),
            );
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_decision(self.sampling_decision);

        let event = StatsigEvent {
            event_name: CONFIG_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(
            self.exposure_time,
            self.user,
            event,
            Some(self.secondary_exposures.unwrap_or_default()),
        )
    }
}
