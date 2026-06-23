use super::{batch::EventBatch, queued_event::QueuedEvent};
use crate::{
    event_logging::statsig_event_internal::StatsigEventInternal, log_d, log_e,
    macros::LOCK_TIMEOUT, read_lock_or_return, write_lock_or_return,
};
use parking_lot::{Mutex, RwLock};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const TAG: &str = stringify!(EventQueue);

// S2SDK-16 fix (exposure-logging path): producers append to the queue on every
// logged exposure. A single global RwLock<VecDeque> made all of them serialize
// on one write lock at high QPS. We stripe the pending buffer across N
// independently locked shards so concurrent producers usually touch different
// shards. 16 shards comfortably exceeds typical server core counts, keeping
// lock collisions rare without wasting memory.
//
// The actual shard count is clamped to `max_pending_events` (see `new`) so we
// never have more shards than the configured capacity, which keeps the
// drop-oldest cap accurate for very small queues.
const PENDING_SHARD_COUNT: usize = 16;

// Each pending event carries a globally-monotonic sequence number assigned at
// enqueue time. Sharding distributes events across stripes for low lock
// contention, but downstream consumers (and our test suites) require events to
// be flushed in the exact order they were logged. We therefore sort the drained
// events by this sequence on the (infrequent, single-threaded) drain path to
// reconstruct global FIFO order without serializing the hot enqueue path.
type SequencedEvent = (u64, QueuedEvent);

pub enum QueueAddResult {
    Noop,
    NeedsFlush,
    NeedsFlushAndDropped(u64),
}

pub enum QueueReconcileResult {
    Success,
    DroppedEvents(u64),
    LockFailure,
}

pub struct EventQueue {
    pub batch_size: usize,
    pub max_pending_batches: usize,

    // Sharded pending-event buffers (see PENDING_SHARD_COUNT). Each add only
    // briefly locks one stripe instead of contending on a single global lock.
    // Events are tagged with a sequence number so global insertion order can be
    // reconstructed on drain.
    pending_shards: Vec<Mutex<VecDeque<SequencedEvent>>>,
    // Approximate total events across all shards. Maintained with relaxed
    // atomics so we can compute capacity / flush signals without locking every
    // shard on the hot path.
    pending_count: AtomicUsize,
    // Round-robin selector spreading writes across shards.
    next_shard: AtomicUsize,
    // Monotonic sequence assigned to each enqueued event to preserve global
    // FIFO order across shards when draining.
    next_seq: AtomicU64,
    batches: RwLock<VecDeque<EventBatch>>,
    max_pending_events: usize,
}

impl EventQueue {
    pub fn new(batch_size: u32, max_queue_size: u32) -> Self {
        let batch_size = batch_size as usize;
        let max_queue_size = max_queue_size as usize;
        let max_pending_events = batch_size * max_queue_size;

        // Never allocate more shards than the capacity. If shards outnumbered
        // max_pending_events, round-robin would place one event per shard and
        // the "don't evict the just-added event" guard (shard.len() > 1) would
        // let pending_count overshoot the cap by up to (shards - cap) before
        // any eviction triggered. Clamping keeps the cap exact for tiny queues
        // (e.g. batch_size=10, max_queue_size=1 => cap 10 < 16 shards). At least
        // one shard so the round-robin modulo is always valid.
        let shard_count = PENDING_SHARD_COUNT.min(max_pending_events).max(1);
        let pending_shards = (0..shard_count)
            .map(|_| Mutex::new(VecDeque::new()))
            .collect();

        Self {
            pending_shards,
            pending_count: AtomicUsize::new(0),
            next_shard: AtomicUsize::new(0),
            next_seq: AtomicU64::new(0),
            batches: RwLock::new(VecDeque::new()),
            batch_size,
            max_pending_batches: max_queue_size,
            max_pending_events,
        }
    }

    pub fn approximate_pending_events_count(&self) -> usize {
        // Lock-free read of the sharded pending total plus the batched events.
        let pending_len = self.pending_count.load(Ordering::Relaxed);
        let batches_len = read_lock_or_return!(TAG, self.batches, 0).len();
        pending_len + (batches_len * self.batch_size)
    }

    pub fn add(&self, pending_event: QueuedEvent) -> QueueAddResult {
        // Pick a shard round-robin so concurrent producers rarely hit the same
        // lock. Only this one stripe is locked, and only for a push_back.
        let shard_idx = self.next_shard.fetch_add(1, Ordering::Relaxed) % self.pending_shards.len();
        let mut shard = match self.pending_shards[shard_idx].try_lock_for(LOCK_TIMEOUT) {
            Some(guard) => guard,
            None => {
                // Mirror the crate lock macros' behavior: a lock timeout under
                // test almost certainly means a deadlock/contention regression
                // in the sharded queue, so fail loudly. In production, log and
                // no-op so we never block the caller.
                if cfg!(test) {
                    panic!("Failed to acquire pending shard lock");
                }
                log_e!(TAG, "Failed to acquire pending shard lock");
                return QueueAddResult::Noop;
            }
        };

        // Stamp the event with a global sequence so drain can restore exact
        // insertion order regardless of which shard it landed in.
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        shard.push_back((seq, pending_event));
        // `total` is this add's unique post-increment ticket value, so exactly
        // every batch_size-th successful add signals a flush - matching the old
        // `len % batch_size == 0` behavior, now without holding a global lock.
        let total = self.pending_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Best-effort drop-oldest overflow guard. To avoid a second global
        // lock we evict from the shard we already hold, so we drop this shard's
        // oldest entry rather than the true global oldest - "oldest" is
        // therefore approximate across shards. This is acceptable for a safety
        // valve that only triggers when the consumer can't keep up.
        //
        // We require `shard.len() > 1` so we never evict the event we just
        // pushed (it sits at the back; the front is an older entry). In normal
        // operation overflow only happens once capacity is far exceeded, where
        // round-robin keeps every shard well-populated, so this reliably drops a
        // genuinely older event. In the degenerate case where this shard holds
        // only the new event, we skip eviction this round and let a fuller shard
        // absorb it on a later add; the queue may then briefly exceed capacity
        // by at most one event per near-empty shard, which is bounded and fine.
        if total > self.max_pending_events && shard.len() > 1 && shard.pop_front().is_some() {
            self.pending_count.fetch_sub(1, Ordering::Relaxed);
            drop(shard);
            return QueueAddResult::NeedsFlushAndDropped(1);
        }
        drop(shard);

        if total.is_multiple_of(self.batch_size) {
            return QueueAddResult::NeedsFlush;
        }

        QueueAddResult::Noop
    }

    pub fn requeue_batch(&self, batch: EventBatch) -> QueueReconcileResult {
        let len = batch.events.len() as u64;
        let mut batches =
            write_lock_or_return!(TAG, self.batches, QueueReconcileResult::DroppedEvents(len));

        if batches.len() > self.max_pending_batches {
            return QueueReconcileResult::DroppedEvents(len);
        }

        log_d!(
            TAG,
            "Requeueing batch with {} events and {} attempts to flush",
            batch.events.len(),
            batch.attempts
        );

        batches.push_back(batch);
        QueueReconcileResult::Success
    }

    pub fn contains_at_least_one_full_batch(&self) -> bool {
        if self.pending_count.load(Ordering::Relaxed) >= self.batch_size {
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
        let mut batches = write_lock_or_return!(TAG, self.batches, VecDeque::new());
        std::mem::take(&mut *batches)
    }

    pub fn take_next_batch(&self) -> Option<EventBatch> {
        let mut batches = write_lock_or_return!(TAG, self.batches, None);
        batches.pop_front()
    }

    pub fn reconcile_batching(&self) -> QueueReconcileResult {
        let mut pending_events: VecDeque<StatsigEventInternal> = self
            .take_all_pending_events()
            .into_iter()
            .map(|evt| evt.into_statsig_event_internal())
            .collect();

        if pending_events.is_empty() {
            return QueueReconcileResult::Success;
        }

        let mut batches =
            write_lock_or_return!(TAG, self.batches, QueueReconcileResult::LockFailure);
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
            return QueueReconcileResult::DroppedEvents(dropped_events_count);
        }

        QueueReconcileResult::Success
    }

    fn take_all_pending_events(&self) -> VecDeque<QueuedEvent> {
        // Atomic all-or-nothing drain. We lock every shard up-front (in fixed
        // index order, so concurrent drains can never deadlock) and only then
        // swap their buffers out. Draining shards one-at-a-time instead would
        // break global FIFO if a later shard's lock timed out: its older events
        // would flush on a subsequent pass, *after* newer events from shards we
        // already drained. Locking all shards first guarantees we either take a
        // globally-consistent snapshot or take nothing and retry next pass.
        let mut guards = Vec::with_capacity(self.pending_shards.len());
        for shard in &self.pending_shards {
            match shard.try_lock_for(LOCK_TIMEOUT) {
                Some(guard) => guards.push(guard),
                None => {
                    // Mirror the crate lock macros: panic under test so a
                    // deadlock surfaces loudly in the concurrency tests.
                    if cfg!(test) {
                        panic!("Failed to lock pending shard for drain");
                    }
                    // In production, drain nothing this pass (dropping the
                    // guards we hold releases them) so ordering is preserved;
                    // the next flush retries.
                    log_e!(
                        TAG,
                        "Failed to lock all pending shards for drain; skipping this pass"
                    );
                    return VecDeque::new();
                }
            }
        }

        // We now hold every shard. Swap each buffer out in O(1) so the locked
        // window stays tiny, then release before the larger merge/sort.
        let mut taken: Vec<VecDeque<SequencedEvent>> = Vec::with_capacity(guards.len());
        let mut drained_count = 0usize;
        for guard in &mut guards {
            let buf = std::mem::take(&mut **guard);
            drained_count += buf.len();
            taken.push(buf);
        }

        // Decrement the counter while we still hold every shard lock so it stays
        // consistent with the (now empty) shard contents at the moment locks are
        // released. No producer can be mid-add here - add() increments
        // pending_count while holding its shard lock and we hold them all - so
        // releasing after this point never exposes an inflated count that could
        // trip spurious evictions / flush signals.
        self.pending_count
            .fetch_sub(drained_count, Ordering::Relaxed);
        drop(guards);

        let mut all: Vec<SequencedEvent> = Vec::with_capacity(drained_count);
        for shard_events in taken {
            all.extend(shard_events);
        }

        // Restore global FIFO order. Sharding lets events land out of order
        // across stripes; sorting by the enqueue sequence here recovers the
        // exact order they were logged in, which downstream batching and
        // consumers rely on. Sequence numbers are unique and monotonic, so this
        // is a total order. This sort only runs on the infrequent drain path,
        // not on the hot enqueue path.
        all.sort_unstable_by_key(|(seq, _)| *seq);
        all.into_iter().map(|(_, event)| event).collect()
    }

    fn create_batches(
        &self,
        mut pending_events: VecDeque<StatsigEventInternal>,
    ) -> Vec<EventBatch> {
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
            exposure_time: 1,
            user: &user_internal,
            queried_gate_name: &gate.name,
            evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
            details: EvaluationDetails::unrecognized_no_data(),
            trigger: ExposureTrigger::Auto,
        };

        let queued_event = enqueue_op.into_queued_event(ForceSampled);

        let result = queue.add(queued_event);

        assert!(matches!(result, QueueAddResult::Noop));
        assert_eq!(queue.pending_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_adding_multiple_to_queue() {
        let (queue, user, gate) = setup(1000, 20);
        let user_internal = StatsigUserInternal::new(&user, None);

        let mut triggered_count = 0;
        for _ in 0..4567 {
            let enqueue_op = EnqueueGateExpoOp {
                exposure_time: 1,
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };

            let result = queue.add(enqueue_op.into_queued_event(ForceSampled));

            if let QueueAddResult::NeedsFlush = result {
                triggered_count += 1;
            }
        }

        assert_eq!(queue.pending_count.load(Ordering::Relaxed), 4567);
        assert_eq!(triggered_count, (4567 / 1000) as usize);
    }

    #[test]
    fn test_take_all_batches() {
        let batch_size = 200;
        let max_pending_batches = 40;

        let (queue, user, gate) = setup(batch_size, max_pending_batches);
        let user_internal = StatsigUserInternal::new(&user, None);

        for _ in 0..4567 {
            let enqueue_op = EnqueueGateExpoOp {
                exposure_time: 1,
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
                exposure_time: 1,
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
            queue
                .batches
                .try_read_for(std::time::Duration::from_secs(5))
                .unwrap()
                .len(),
            (max_pending_batches - 1) as usize
        ); // max minus the one we just took
    }

    // S2SDK-16 regression test: many producer threads hammer `add` at once
    // (simulating the high-QPS exposure-logging path). Asserts the sharded
    // queue neither loses nor duplicates events under contention.
    #[test]
    fn test_concurrent_add_preserves_all_events() {
        let batch_size = 1000;
        // Large capacity so the drop-oldest safety valve never triggers here.
        let max_pending_batches = 1000;
        let (queue, user, gate) = setup(batch_size, max_pending_batches);
        let queue = std::sync::Arc::new(queue);
        let user_internal = StatsigUserInternal::new(&user, None);

        let thread_count = 16;
        let per_thread = 5000;

        std::thread::scope(|s| {
            for _ in 0..thread_count {
                let queue = queue.clone();
                let user_internal = &user_internal;
                let gate = &gate;
                s.spawn(move || {
                    for _ in 0..per_thread {
                        let enqueue_op = EnqueueGateExpoOp {
                            exposure_time: 1,
                            user: user_internal,
                            queried_gate_name: &gate.name,
                            evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                            details: EvaluationDetails::unrecognized_no_data(),
                            trigger: ExposureTrigger::Auto,
                        };
                        queue.add(enqueue_op.into_queued_event(ForceSampled));
                    }
                });
            }
        });

        let total = thread_count * per_thread;
        assert_eq!(queue.pending_count.load(Ordering::Relaxed), total);

        // After reconcile + drain, every enqueued event is accounted for in a
        // batch and the pending counter is back to zero.
        queue.reconcile_batching();
        let batches = queue.take_all_batches();
        let event_total: usize = batches.iter().map(|b| b.events.len()).sum();
        assert_eq!(event_total, total);
        assert_eq!(queue.pending_count.load(Ordering::Relaxed), 0);
    }

    // Verifies the drop-oldest overflow guard still bounds the queue after the
    // sharding refactor: enqueue far past capacity, then confirm the retained
    // event count never exceeds max_pending_events and the counter stays
    // consistent with what is actually held.
    #[test]
    fn test_concurrent_add_respects_capacity() {
        let batch_size = 100;
        let max_pending_batches = 4; // max_pending_events = 400
        let (queue, user, gate) = setup(batch_size, max_pending_batches);
        let queue = std::sync::Arc::new(queue);
        let user_internal = StatsigUserInternal::new(&user, None);
        let max_pending_events = (batch_size * max_pending_batches) as usize;

        let thread_count = 8;
        let per_thread = 5000;

        std::thread::scope(|s| {
            for _ in 0..thread_count {
                let queue = queue.clone();
                let user_internal = &user_internal;
                let gate = &gate;
                s.spawn(move || {
                    for _ in 0..per_thread {
                        let enqueue_op = EnqueueGateExpoOp {
                            exposure_time: 1,
                            user: user_internal,
                            queried_gate_name: &gate.name,
                            evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                            details: EvaluationDetails::unrecognized_no_data(),
                            trigger: ExposureTrigger::Auto,
                        };
                        queue.add(enqueue_op.into_queued_event(ForceSampled));
                    }
                });
            }
        });

        // The reported pending count must never exceed capacity...
        let reported = queue.pending_count.load(Ordering::Relaxed);
        assert!(
            reported <= max_pending_events,
            "pending_count {reported} exceeded capacity {max_pending_events}"
        );

        // ...and must match the number of events actually retained in shards.
        let drained = queue.take_all_pending_events();
        assert_eq!(drained.len(), reported);
        assert!(drained.len() <= max_pending_events);
    }

    // S2SDK-16 regression test: sharding must not reorder events. The rulesets
    // suites assert the Nth logged exposure maps to the Nth evaluation, so the
    // queue must drain in exact insertion order even though adds are striped
    // round-robin across shards. We tag each event with a monotonically
    // increasing exposure_time (which becomes the event `time`) and verify the
    // drained sequence is strictly increasing.
    #[test]
    fn test_drain_preserves_fifo_order() {
        let event_count = 5000u64; // well past PENDING_SHARD_COUNT to span all shards
        let (queue, user, gate) = setup(event_count as u32 + 1, 10);
        let user_internal = StatsigUserInternal::new(&user, None);

        for i in 0..event_count {
            let enqueue_op = EnqueueGateExpoOp {
                exposure_time: i,
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };
            queue.add(enqueue_op.into_queued_event(ForceSampled));
        }

        let drained: Vec<u64> = queue
            .take_all_pending_events()
            .into_iter()
            .map(|evt| evt.into_statsig_event_internal().time)
            .collect();

        assert_eq!(drained.len(), event_count as usize);
        let expected: Vec<u64> = (0..event_count).collect();
        assert_eq!(drained, expected, "events drained out of insertion order");
    }

    // S2SDK-16 review fix: when the configured capacity is smaller than the
    // default shard count (batch_size=10, max_queue_size=1 => cap 10 < 16
    // shards), shards are clamped to the capacity so pending_count can never
    // overshoot max_pending_events.
    #[test]
    fn test_small_capacity_respects_cap() {
        let batch_size = 10;
        let max_queue_size = 1;
        let (queue, user, gate) = setup(batch_size, max_queue_size);
        let user_internal = StatsigUserInternal::new(&user, None);
        let max_pending_events = (batch_size * max_queue_size) as usize;

        for _ in 0..1000 {
            let enqueue_op = EnqueueGateExpoOp {
                exposure_time: 1,
                user: &user_internal,
                queried_gate_name: &gate.name,
                evaluation: gate.__evaluation.as_ref().map(Cow::Borrowed),
                details: EvaluationDetails::unrecognized_no_data(),
                trigger: ExposureTrigger::Auto,
            };
            queue.add(enqueue_op.into_queued_event(ForceSampled));
        }

        let reported = queue.pending_count.load(Ordering::Relaxed);
        assert!(
            reported <= max_pending_events,
            "pending_count {reported} overshot cap {max_pending_events}"
        );
        let drained = queue.take_all_pending_events();
        assert_eq!(drained.len(), reported);
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
