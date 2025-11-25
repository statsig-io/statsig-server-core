use super::{
    queued_config_expo::QueuedConfigExposureEvent,
    queued_experiment_expo::QueuedExperimentExposureEvent, queued_expo::QueuedExposureEvent,
    queued_gate_expo::QueuedGateExposureEvent,
    queued_layer_param_expo::QueuedLayerParamExposureEvent,
};
use crate::{
    evaluation::evaluation_types::ExtraExposureInfo,
    event_logging::{
        exposure_sampling::{EvtSamplingDecision, ExposureSamplingKey},
        statsig_event_internal::StatsigEventInternal,
    },
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
        }
    }
}
