use crate::event_logging::{
    exposure_sampling::EvtSamplingDecision, statsig_event_internal::StatsigEventInternal,
};

use super::{
    queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure},
    queued_gate_expo::EnqueueGateExpoOp,
};

pub struct EnqueuePassthroughOp {
    pub event: StatsigEventInternal,
}

impl EnqueueOperation for EnqueuePassthroughOp {
    fn as_exposure(&self) -> Option<&impl QueuedExposure> {
        None::<&EnqueueGateExpoOp>
    }

    fn into_queued_event(self, _sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        QueuedEvent::Passthrough(self.event)
    }
}
