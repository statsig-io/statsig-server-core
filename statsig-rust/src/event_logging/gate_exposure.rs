use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::event_logging::exposure_utils::{
    get_metadata_with_details, get_statsig_metadata_with_sampling_details, make_exposure_key,
};
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::sampling_processor::SamplingDecision;
use crate::statsig_user_internal::StatsigUserLoggable;

pub const GATE_EXPOSURE_EVENT_NAME: &str = "statsig::gate_exposure";

pub struct GateExposure {
    pub user: StatsigUserLoggable,
    pub gate_name: String,
    pub value: bool,
    pub rule_id: Option<String>,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub evaluation_details: EvaluationDetails,
    pub version: Option<u32>,
    pub is_manual_exposure: bool,
    pub sampling_details: SamplingDecision,
    pub override_config_name: Option<String>,
}

impl StatsigExposure for GateExposure {
    fn make_dedupe_key(&self) -> String {
        make_exposure_key(&self.user, &self.gate_name, self.rule_id.as_ref(), None)
    }

    fn to_internal_event(self) -> StatsigEventInternal {
        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("gate".into(), self.gate_name);
        metadata.insert("gateValue".into(), self.value.to_string());
        metadata.insert("ruleID".into(), self.rule_id.unwrap_or_default());
        if self.is_manual_exposure {
            metadata.insert("isManualExposure".into(), "true".into());
        }
        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert("overrideConfigName".into(), override_config_name);
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_details(self.sampling_details);

        let event = StatsigEvent {
            event_name: GATE_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(self.user, event, self.secondary_exposures)
    }
}
