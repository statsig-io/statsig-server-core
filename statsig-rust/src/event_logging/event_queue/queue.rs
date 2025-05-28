use crate::{
    event_logging::statsig_event_internal::StatsigEventInternal, log_d, read_lock_or_return,
    write_lock_or_return,
};
use std::{collections::VecDeque, sync::RwLock};

use super::{batch::EventBatch, queued_event::QueuedEvent};

const TAG: &str = stringify!(EventQueue);

pub enum QueueResult {
    Success,
    DroppedEvents(u64),
}

pub struct EventQueue {
    pub batch_size: usize,
    pub max_pending_batches: usize,

    pending_events: RwLock<Vec<QueuedEvent>>,
    batches: RwLock<VecDeque<EventBatch>>,
}

impl EventQueue {
    pub fn new(batch_size: u32, max_queue_size: u32) -> Self {
        Self {
            pending_events: RwLock::new(Vec::new()),
            batches: RwLock::new(VecDeque::new()),
            batch_size: batch_size as usize,
            max_pending_batches: max_queue_size as usize,
        }
    }

    pub fn approximate_pending_events_count(&self) -> usize {
        let pending_len = read_lock_or_return!(TAG, self.pending_events, 0).len();
        let batches_len = read_lock_or_return!(TAG, self.batches, 0).len();
        pending_len + (batches_len * self.batch_size)
    }

    pub fn add(&self, pending_event: QueuedEvent) -> bool {
        let mut pending_events = write_lock_or_return!(TAG, self.pending_events, false);
        pending_events.push(pending_event);
        pending_events.len() % self.batch_size == 0
    }

    pub fn requeue_batch(&self, batch: EventBatch) -> QueueResult {
        let len = batch.events.len() as u64;
        let mut batches = write_lock_or_return!(TAG, self.batches, QueueResult::DroppedEvents(len));

        if batches.len() > self.max_pending_batches {
            return QueueResult::DroppedEvents(len);
        }

        log_d!(
            TAG,
            "Requeueing batch with {} events and {} attempts to flush",
            batch.events.len(),
            batch.attempts
        );

        batches.push_back(batch);
        QueueResult::Success
    }

    pub fn contains_at_least_one_full_batch(&self) -> bool {
        let pending_events_count = self.pending_events.read().map(|e| e.len()).unwrap_or(0);
        if pending_events_count >= self.batch_size {
            return true;
        }

        let batches = read_lock_or_return!(TAG, self.batches, false);
        for batch in batches.iter() {
            if batch.events.len() >= self.batch_size {
                return true;
            }
        }

        false
    }

    pub fn take_all_batches(&self) -> VecDeque<EventBatch> {
        let mut batches = self.batches.write().unwrap();
        std::mem::take(&mut *batches)
    }

    pub fn take_next_batch(&self) -> Option<EventBatch> {
        let mut batches = self.batches.write().unwrap();
        batches.pop_front()
    }

    pub fn reconcile_batching(&self) -> QueueResult {
        let mut pending_events: Vec<StatsigEventInternal> = self
            .take_all_pending_events()
            .into_iter()
            .map(|evt| evt.into_statsig_event_internal())
            .collect();

        if pending_events.is_empty() {
            return QueueResult::Success;
        }

        let mut batches = self.batches.write().unwrap();
        let old_batches = std::mem::take(&mut *batches);

        let (full_batches, partial_batches): (VecDeque<_>, VecDeque<_>) = old_batches
            .into_iter()
            .partition(|batch| batch.events.len() >= self.batch_size);

        for batch in partial_batches {
            pending_events.extend(batch.events);
        }

        let new_batches = self.create_batches(pending_events);

        batches.extend(full_batches);
        batches.extend(new_batches);

        let dropped_events_count = self.clamp_batches(&mut batches);
        if dropped_events_count > 0 {
            return QueueResult::DroppedEvents(dropped_events_count);
        }

        QueueResult::Success
    }

    fn take_all_pending_events(&self) -> Vec<QueuedEvent> {
        let mut pending_events = write_lock_or_return!(TAG, self.pending_events, Vec::new());
        std::mem::take(&mut *pending_events)
    }

    fn create_batches(&self, mut pending_events: Vec<StatsigEventInternal>) -> Vec<EventBatch> {
        let mut batches = Vec::new();
        while !pending_events.is_empty() {
            let drain_count = self.batch_size.min(pending_events.len());
            let chunk = pending_events.drain(..drain_count).collect::<Vec<_>>();
            batches.push(EventBatch::new(chunk));
        }

        batches
    }

    fn clamp_batches(&self, batches: &mut VecDeque<EventBatch>) -> u64 {
        if batches.len() <= self.max_pending_batches {
            return 0;
        }

        let mut dropped_events_count = 0;
        while batches.len() > self.max_pending_batches {
            if let Some(batch) = batches.pop_front() {
                dropped_events_count += batch.events.len() as u64;
            }
        }

        dropped_events_count
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::event_logging::event_queue::queued_event::EnqueueOperation;
    use crate::event_logging::event_queue::queued_gate_expo::EnqueueGateExpoOp;
    use crate::event_logging::exposure_sampling::EvtSamplingDecision::ForceSampled;
    use crate::{
        event_logging::event_logger::ExposureTrigger, statsig_types::FeatureGate,
        user::StatsigUserInternal, EvaluationDetails, StatsigUser,
    };

    #[test]
    fn test_adding_single_to_queue() {
        let (queue, user, gate) = setup(10, 10);
        let user_internal = StatsigUserInternal::new(&user, None);

        let enqueue_op = EnqueueGateExpoOp {
            user: &user_internal,
            queried_gate_name: &gate.name,
            evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
            details: EvaluationDetails::unrecognized_no_data(),
            trigger: ExposureTrigger::Auto,
        };

        let queued_event = enqueue_op.into_queued_event(ForceSampled);

        let has_exceeded_limit = queue.add(queued_event);

        assert!(!has_exceeded_limit);
        assert_eq!(queue.pending_events.read().unwrap().len(), 1);
    }

    #[test]
    fn test_adding_multiple_to_queue() {
        let (queue, user, gate) = setup(100, 20);
        let user_internal = StatsigUserInternal::new(&user, None);

        let mut triggered_count = 0;
        for _ in 0..4567 {
            let enqueue_op = EnqueueGateExpoOp {
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };

            let did_trigger = queue.add(enqueue_op.into_queued_event(ForceSampled));

            if did_trigger {
                triggered_count += 1;
            }
        }

        assert_eq!(queue.pending_events.read().unwrap().len(), 4567);
        assert_eq!(triggered_count, (4567 / 100) as usize);
    }

    #[test]
    fn test_take_all_batches() {
        let batch_size = 200;
        let max_pending_batches = 40;

        let (queue, user, gate) = setup(batch_size, max_pending_batches);
        let user_internal = StatsigUserInternal::new(&user, None);

        for _ in 0..4567 {
            let enqueue_op = EnqueueGateExpoOp {
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };
            queue.add(enqueue_op.into_queued_event(ForceSampled));
        }

        queue.reconcile_batching();
        let batches = queue.take_all_batches();
        assert_eq!(batches.len(), (4567.0 / batch_size as f64).ceil() as usize,);
    }

    #[test]
    fn test_take_next_batch() {
        let batch_size = 200;
        let max_pending_batches = 20;

        let (queue, user, gate) = setup(batch_size, max_pending_batches);
        let user_internal = StatsigUserInternal::new(&user, None);

        for _ in 0..4567 {
            let enqueue_op = EnqueueGateExpoOp {
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };
            queue.add(enqueue_op.into_queued_event(ForceSampled));
        }

        queue.reconcile_batching();
        let batch = queue.take_next_batch();
        assert_eq!(batch.unwrap().events.len(), batch_size as usize);

        assert_eq!(
            queue.batches.read().unwrap().len(),
            (max_pending_batches - 1) as usize
        ); // max minus the one we just took
    }

    fn setup(batch_size: u32, max_queue_size: u32) -> (EventQueue, StatsigUser, FeatureGate) {
        let queue = EventQueue::new(batch_size, max_queue_size);
        let user = StatsigUser::with_user_id("user-id");
        let gate = FeatureGate {
            name: "gate-name".into(),
            value: true,
            rule_id: "rule-id".into(),
            id_type: "user-id".into(),
            details: EvaluationDetails::unrecognized_no_data(),
            __evaluation: None,
        };
        (queue, user, gate)
    }
}
