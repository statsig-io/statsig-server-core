use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::BaseEvaluation;
use crate::event_logging::exposure_utils::{get_metadata_with_details, make_exposure_key};
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::statsig_user_internal::StatsigUserInternal;

pub const CONFIG_EXPOSURE_EVENT_NAME: &str = "statsig::config_exposure";

pub struct ConfigExposure {
    pub user: StatsigUserInternal,
    pub config_name: String,
    pub evaluation: Option<BaseEvaluation>,
    pub evaluation_details: EvaluationDetails,
}

impl StatsigExposure for ConfigExposure {
    fn make_dedupe_key(&self) -> String {
        // todo: fill exposure key
        make_exposure_key(&self.user.user_data, &self.config_name, None)
    }

    fn to_internal_event(self) -> StatsigEventInternal {
        let (rule_id, secondary_exposures) = match self.evaluation {
            Some(evaluation) => (evaluation.rule_id, Some(evaluation.secondary_exposures)),
            None => ("".into(), None),
        };

        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.config_name);
        metadata.insert("ruleID".into(), rule_id);

        let event = StatsigEvent {
            event_name: CONFIG_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
        };

        StatsigEventInternal::new(
            self.user,
            event,
            Some(secondary_exposures.unwrap_or_default()),
        )
    }
}
