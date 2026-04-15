use crate::{
    event_logging::{
        exposure_sampling::{EvtSamplingDecision, ExposureSamplingKey},
        exposure_utils::get_statsig_metadata_with_sampling_decision,
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, GATE_EXPOSURE_EVENT_NAME},
    },
    user::StatsigUserLoggable,
    SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};
use crate::event_logging::statsig_event::string_metadata_to_value_metadata;
use std::collections::HashMap;

pub struct EnqueueSecondaryExposureAsPrimaryOp {
    pub user: StatsigUserLoggable,
    pub secondary_exposure: SecondaryExposure,
    pub exposure_time: u64,
}

impl EnqueueOperation for EnqueueSecondaryExposureAsPrimaryOp {
    fn as_exposure(&self) -> Option<&impl QueuedExposure<'_>> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        QueuedEvent::SecondaryExposureAsPrimary(QueuedSecondaryExposureAsPrimaryEvent {
            user: self.user,
            secondary_exposure: self.secondary_exposure,
            exposure_time: self.exposure_time,
            sampling_decision,
        })
    }
}

pub struct QueuedSecondaryExposureAsPrimaryEvent {
    pub user: StatsigUserLoggable,
    pub secondary_exposure: SecondaryExposure,
    pub exposure_time: u64,
    pub sampling_decision: EvtSamplingDecision,
}

impl QueuedSecondaryExposureAsPrimaryEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let metadata = HashMap::from([
            (
                "gate".into(),
                self.secondary_exposure.gate.unperformant_to_string(),
            ),
            (
                "gateValue".into(),
                self.secondary_exposure.gate_value.unperformant_to_string(),
            ),
            (
                "ruleID".into(),
                self.secondary_exposure.rule_id.unperformant_to_string(),
            ),
        ]);

        let event = StatsigEvent {
            event_name: GATE_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(string_metadata_to_value_metadata(metadata)),
            statsig_metadata: Some(get_statsig_metadata_with_sampling_decision(
                self.sampling_decision,
            )),
        };

        StatsigEventInternal::new(self.exposure_time, self.user, event, Some(Vec::new()))
    }
}

impl<'a> QueuedExposure<'a> for EnqueueSecondaryExposureAsPrimaryOp {
    fn create_exposure_sampling_key(&self) -> ExposureSamplingKey {
        ExposureSamplingKey {
            spec_name_hash: self.secondary_exposure.gate.hash,
            rule_id_hash: self.secondary_exposure.rule_id.hash,
            user_values_hash: self.user.data.create_exposure_dedupe_user_hash(None),
            additional_hash: (self.secondary_exposure.gate_value.as_str() == "true") as u64,
        }
    }

    fn get_rule_id_ref(&'a self) -> &'a str {
        self.secondary_exposure.rule_id.as_str()
    }

    fn get_extra_exposure_info_ref(
        &'a self,
    ) -> Option<&'a crate::evaluation::evaluation_types::ExtraExposureInfo> {
        None
    }
}
