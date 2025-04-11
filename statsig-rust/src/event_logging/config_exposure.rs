use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::BaseEvaluation;
use crate::event_logging::exposure_utils::{
    get_metadata_with_details, get_statsig_metadata_with_sampling_details, make_exposure_key,
};
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::sampling_processor::SamplingDecision;
use crate::statsig_user_internal::StatsigUserLoggable;

pub const CONFIG_EXPOSURE_EVENT_NAME: &str = "statsig::config_exposure";

pub struct ConfigExposure {
    pub user: StatsigUserLoggable,
    pub config_name: String,
    pub evaluation: Option<BaseEvaluation>,
    pub evaluation_details: EvaluationDetails,
    pub rule_passed: Option<bool>,
    pub version: Option<u32>,
    pub is_manual_exposure: bool,
    pub sampling_details: SamplingDecision,
    pub override_config_name: Option<String>,
}

impl StatsigExposure for ConfigExposure {
    fn make_dedupe_key(&self) -> String {
        let rule_id = self.evaluation.as_ref().map(|eval| &eval.rule_id);
        make_exposure_key(&self.user, &self.config_name, rule_id, None)
    }

    fn to_internal_event(self) -> StatsigEventInternal {
        let (rule_id, secondary_exposures) = match self.evaluation {
            Some(evaluation) => (evaluation.rule_id, Some(evaluation.secondary_exposures)),
            None => (String::new(), None),
        };

        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.config_name);
        metadata.insert("ruleID".into(), rule_id);
        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }
        if let Some(rule_passed) = self.rule_passed {
            metadata.insert("rulePassed".into(), rule_passed.to_string());
        }
        if self.is_manual_exposure {
            metadata.insert("isManualExposure".into(), "true".into());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert("overrideConfigName".into(), override_config_name);
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_details(self.sampling_details);

        let event = StatsigEvent {
            event_name: CONFIG_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(
            self.user,
            event,
            Some(secondary_exposures.unwrap_or_default()),
        )
    }
}
