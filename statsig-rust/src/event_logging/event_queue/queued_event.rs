use super::{
    queued_config_expo::QueuedConfigExposureEvent,
    queued_experiment_expo::QueuedExperimentExposureEvent, queued_expo::QueuedExposureEvent,
    queued_gate_expo::QueuedGateExposureEvent,
    queued_layer_param_expo::QueuedLayerParamExposureEvent,
    queued_secondary_expo::QueuedSecondaryExposureAsPrimaryEvent,
};
use crate::{
    evaluation::evaluation_types::ExtraExposureInfo,
    event_logging::{
        exposure_sampling::{EvtSamplingDecision, ExposureSamplingKey},
        statsig_event_internal::StatsigEventInternal,
    },
    user::StatsigUserLoggable,
    SecondaryExposure,
};

pub trait EnqueueOperation {
    fn as_exposure(&self) -> Option<&impl QueuedExposure<'_>>;
    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent;
}

pub trait QueuedExposure<'a> {
    fn create_exposure_sampling_key(&self) -> ExposureSamplingKey;
    fn get_rule_id_ref(&'a self) -> &'a str;
    fn get_extra_exposure_info_ref(&'a self) -> Option<&'a ExtraExposureInfo>;
}

pub enum QueuedEvent {
    Exposure(QueuedExposureEvent),
    Passthrough(StatsigEventInternal),

    // Deprecated - Remove when rolled into QueuedExposureEvent
    GateExposure(QueuedGateExposureEvent),
    ConfigExposure(QueuedConfigExposureEvent),
    ExperimentExposure(QueuedExperimentExposureEvent),
    LayerParamExposure(QueuedLayerParamExposureEvent),
    SecondaryExposureAsPrimary(QueuedSecondaryExposureAsPrimaryEvent),
}

impl QueuedEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        match self {
            QueuedEvent::Exposure(event) => event.into_statsig_event_internal(),
            QueuedEvent::Passthrough(event) => event,

            QueuedEvent::GateExposure(event) => event.into_statsig_event_internal(),
            QueuedEvent::ConfigExposure(event) => event.into_statsig_event_internal(),
            QueuedEvent::ExperimentExposure(event) => event.into_statsig_event_internal(),
            QueuedEvent::LayerParamExposure(event) => event.into_statsig_event_internal(),
            QueuedEvent::SecondaryExposureAsPrimary(event) => event.into_statsig_event_internal(),
        }
    }

    pub fn exposure_time(&self) -> Option<u64> {
        match self {
            QueuedEvent::Exposure(event) => Some(event.data.exposure_time),
            QueuedEvent::GateExposure(event) => Some(event.exposure_time),
            QueuedEvent::ConfigExposure(event) => Some(event.exposure_time),
            QueuedEvent::ExperimentExposure(event) => Some(event.exposure_time),
            QueuedEvent::LayerParamExposure(event) => Some(event.exposure_time),
            QueuedEvent::SecondaryExposureAsPrimary(event) => Some(event.exposure_time),
            QueuedEvent::Passthrough(_) => None,
        }
    }

    pub fn user_for_secondary_exposures(&self) -> Option<StatsigUserLoggable> {
        match self {
            QueuedEvent::Exposure(event) => Some(event.user.clone()),
            QueuedEvent::GateExposure(event) => Some(event.user.clone()),
            QueuedEvent::ConfigExposure(event) => Some(event.user.clone()),
            QueuedEvent::ExperimentExposure(event) => Some(event.user.clone()),
            QueuedEvent::LayerParamExposure(event) => Some(event.user.clone()),
            QueuedEvent::SecondaryExposureAsPrimary(event) => Some(event.user.clone()),
            QueuedEvent::Passthrough(_) => None,
        }
    }

    pub fn take_secondary_exposures_for_primary_logging(&mut self) -> Vec<SecondaryExposure> {
        match self {
            QueuedEvent::Exposure(event) => event.take_secondary_exposures_for_primary_logging(),
            QueuedEvent::GateExposure(event) => {
                take_non_empty_secondary_exposures(&mut event.secondary_exposures)
            }
            QueuedEvent::ConfigExposure(event) => {
                take_non_empty_secondary_exposures(&mut event.secondary_exposures)
            }
            QueuedEvent::ExperimentExposure(event) => {
                take_non_empty_secondary_exposures(&mut event.secondary_exposures)
            }
            QueuedEvent::LayerParamExposure(event) => {
                take_non_empty_secondary_exposures(&mut event.secondary_exposures)
            }
            QueuedEvent::Passthrough(_) | QueuedEvent::SecondaryExposureAsPrimary(_) => Vec::new(),
        }
    }
}

pub(crate) fn take_non_empty_secondary_exposures(
    secondary_exposures: &mut Option<Vec<SecondaryExposure>>,
) -> Vec<SecondaryExposure> {
    match secondary_exposures.take() {
        Some(exposures) if !exposures.is_empty() => {
            *secondary_exposures = Some(Vec::new());
            exposures
        }
        original => {
            *secondary_exposures = original;
            Vec::new()
        }
    }
}
