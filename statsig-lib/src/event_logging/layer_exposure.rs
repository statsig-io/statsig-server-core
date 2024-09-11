use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::LayerEvaluation;
use crate::event_logging::exposure_utils::{get_metadata_with_details, make_exposure_key};
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::statsig_user_internal::StatsigUserInternal;

pub const LAYER_EXPOSURE_EVENT_NAME: &str = "statsig::layer_exposure";

pub struct LayerExposure {
    pub user: StatsigUserInternal,
    pub layer_name: String,
    pub parameter_name: String,
    pub evaluation: Option<LayerEvaluation>,
    pub evaluation_details: EvaluationDetails,
}

impl StatsigExposure for LayerExposure {
    fn make_dedupe_key(&self) -> String {
        let rule_id = self.evaluation.as_ref().map(|eval| &eval.base.base.rule_id);

        // todo: Node dedupes on all metadata values. Important?
        make_exposure_key(&self.user.user_data, &self.layer_name, rule_id, Some(vec![self.parameter_name.clone()]))
    }

    fn to_internal_event(self) -> StatsigEventInternal {
        let (rule_id, secondary_exposures, is_explicit, allocated_experiment) =
            match self.evaluation {
                Some(evaluation) => {
                    let is_explicit = evaluation
                        .explicit_parameters
                        .contains(&self.parameter_name);

                    let mut secondary_exposures = evaluation
                        .undelegated_secondary_exposures
                        .unwrap_or_else(Vec::new);

                    let mut allocated_experiment = "".into();

                    if is_explicit {
                        allocated_experiment = evaluation.allocated_experiment_name;
                        secondary_exposures = evaluation.base.base.secondary_exposures;
                    }

                    (
                        evaluation.base.base.rule_id,
                        Some(secondary_exposures),
                        is_explicit,
                        allocated_experiment,
                    )
                }
                None => ("".into(), None, false, "".into()),
            };

        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.layer_name);
        metadata.insert("ruleID".into(), rule_id);
        metadata.insert("allocatedExperiment".into(), allocated_experiment);
        metadata.insert("parameterName".into(), self.parameter_name);
        metadata.insert("isExplicitParameter".into(), is_explicit.to_string());

        let event = StatsigEvent {
            event_name: LAYER_EXPOSURE_EVENT_NAME.into(),
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
