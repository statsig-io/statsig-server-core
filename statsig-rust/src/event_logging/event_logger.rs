use super::{
    event_queue::{
        batch::EventBatch,
        queue::{EventQueue, QueueResult},
        queued_event::{EnqueueOperation, QueuedEvent},
    },
    exposure_sampling::ExposureSampling,
    flush_interval::FlushInterval,
    flush_type::FlushType,
    statsig_event_internal::StatsigEventInternal,
};
use crate::{
    event_logging::event_logger_constants::EventLoggerConstants,
    log_d, log_e, log_w,
    networking::NetworkError,
    observability::ops_stats::{OpsStatsForInstance, OPS_STATS},
    statsig_metadata::StatsigMetadata,
    write_lock_or_noop, EventLoggingAdapter, StatsigErr, StatsigOptions, StatsigRuntime,
};
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::{Notify, Semaphore};

const BG_TASK_TAG: &str = "EVENT_LOGGER_BG_TASK";
const DEFAULT_BATCH_SIZE: u32 = 2000;
const DEFAULT_PENDING_BATCH_MAX: u32 = 100;
const MAX_LIMIT_FLUSH_TASKS: usize = 5;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExposureTrigger {
    Auto,
    Manual,
}

const TAG: &str = stringify!(EvtLogger);

pub struct EventLogger {
    queue: EventQueue,
    options: Arc<StatsigOptions>,
    logging_adapter: Arc<dyn EventLoggingAdapter>,
    event_sampler: ExposureSampling,
    non_exposed_checks: RwLock<HashMap<String, u64>>,
    limit_flush_notify: Notify,
    limit_flush_semaphore: Arc<Semaphore>,
    flush_interval: FlushInterval,
    shutdown_notify: Notify,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl EventLogger {
    pub fn new(
        sdk_key: &str,
        options: &Arc<StatsigOptions>,
        event_logging_adapter: &Arc<dyn EventLoggingAdapter>,
        statsig_rt: &Arc<StatsigRuntime>,
    ) -> Arc<Self> {
        let me = Arc::new(Self {
            queue: EventQueue::new(
                options
                    .event_logging_max_queue_size
                    .unwrap_or(DEFAULT_BATCH_SIZE),
                options
                    .event_logging_max_pending_batch_queue_size
                    .unwrap_or(DEFAULT_PENDING_BATCH_MAX),
            ),
            event_sampler: ExposureSampling::new(sdk_key),
            flush_interval: FlushInterval::new(),
            options: options.clone(),
            logging_adapter: event_logging_adapter.clone(),
            non_exposed_checks: RwLock::new(HashMap::new()),
            shutdown_notify: Notify::new(),
            limit_flush_notify: Notify::new(),
            limit_flush_semaphore: Arc::new(Semaphore::new(MAX_LIMIT_FLUSH_TASKS)),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        });

        me.spawn_background_task(statsig_rt);
        me
    }

    pub fn enqueue(self: &Arc<Self>, operation: impl EnqueueOperation) {
        if self.options.disable_all_logging == Some(true) {
            return;
        }

        let decision = self.event_sampler.get_sampling_decision(&operation);
        if !decision.should_log() {
            return;
        }

        let pending_event = operation.into_queued_event(decision);
        if self.queue.add(pending_event) {
            self.limit_flush_notify.notify_one();
        }
    }

    pub fn increment_non_exposure_checks(&self, name: &str) {
        let mut non_exposed_checks = write_lock_or_noop!(TAG, self.non_exposed_checks);

        match non_exposed_checks.get_mut(name) {
            Some(count) => *count += 1,
            None => {
                non_exposed_checks.insert(name.into(), 1);
            }
        }
    }

    pub async fn flush_all_pending_events(&self) -> Result<(), StatsigErr> {
        self.try_flush_all_pending_events(FlushType::Manual).await
    }

    pub async fn shutdown(&self) -> Result<(), StatsigErr> {
        let result = self.try_flush_all_pending_events(FlushType::Shutdown).await;
        self.shutdown_notify.notify_one();
        result
    }

    pub fn force_shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    fn spawn_background_task(self: &Arc<Self>, rt: &Arc<StatsigRuntime>) {
        let me = self.clone();
        let rt_clone = rt.clone();

        rt.spawn(BG_TASK_TAG, |rt_shutdown_notify| async move {
            let tick_interval_ms = EventLoggerConstants::tick_interval_ms();
            let tick_interval = Duration::from_millis(tick_interval_ms);

            loop {
                let can_limit_flush = me.flush_interval.has_completely_recovered_from_backoff();

                tokio::select! {
                    () = tokio::time::sleep(tick_interval) => {
                        me.try_scheduled_flush().await;
                    }
                    () = rt_shutdown_notify.notified() => {
                        return; // Runtime Shutdown
                    }
                    _ = me.shutdown_notify.notified() => {
                        return; // EvtLogger Shutdown
                    }
                    _ = me.limit_flush_notify.notified(), if can_limit_flush => {
                        Self::spawn_new_limit_flush_task(&me, &rt_clone);
                    }
                }

                me.event_sampler.try_reset_all_sampling();
            }
        });
    }

    fn spawn_new_limit_flush_task(inst: &Arc<Self>, rt: &Arc<StatsigRuntime>) {
        let permit = match inst.limit_flush_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let me = inst.clone();
        rt.spawn(BG_TASK_TAG, |_| async move {
            log_d!(TAG, "Attempting limit flush");
            if !me.flush_next_batch(FlushType::Limit).await {
                return;
            }

            loop {
                if !me.flush_interval.has_completely_recovered_from_backoff() {
                    break;
                }

                if !me.queue.contains_at_least_one_full_batch() {
                    break;
                }

                if !me.flush_next_batch(FlushType::Limit).await {
                    break;
                }
            }

            drop(permit);
        });
    }

    async fn try_flush_all_pending_events(&self, flush_type: FlushType) -> Result<(), StatsigErr> {
        self.prepare_event_queue_for_flush(flush_type);

        let batches = self.queue.take_all_batches();

        let results = futures::future::join_all(batches.into_iter().map(|mut batch| async {
            match self.log_batch(&mut batch, flush_type).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    if flush_type == FlushType::Manual {
                        self.flush_interval.adjust_for_failure();
                        self.try_requeue_failed_batch(&e, batch, flush_type);
                        return Err(e);
                    }

                    self.drop_events_for_failure(&e, batch, flush_type);
                    Err(e)
                }
            }
        }))
        .await;

        results.into_iter().find(|r| r.is_err()).unwrap_or(Ok(()))
    }

    async fn try_scheduled_flush(&self) {
        if !self.flush_interval.has_cooled_from_most_recent_failure() {
            return;
        }

        let should_flush_by_time = self.flush_interval.has_waited_max_allowed_interval();
        let should_flush_by_size = self.queue.contains_at_least_one_full_batch();

        if !should_flush_by_time && !should_flush_by_size {
            return;
        }

        self.flush_interval.mark_scheduled_flush_attempt();

        self.flush_next_batch(if should_flush_by_size {
            FlushType::ScheduledFullBatch
        } else {
            FlushType::ScheduledMaxTime
        })
        .await;
    }

    async fn flush_next_batch(&self, flush_type: FlushType) -> bool {
        self.prepare_event_queue_for_flush(flush_type);

        let mut batch = match self.queue.take_next_batch() {
            Some(batch) => batch,
            None => return false,
        };

        let error = match self.log_batch(&mut batch, flush_type).await {
            Err(e) => e,
            Ok(()) => {
                self.flush_interval.adjust_for_success();
                return true;
            }
        };

        self.flush_interval.adjust_for_failure();
        self.try_requeue_failed_batch(&error, batch, flush_type);

        false
    }

    async fn log_batch(
        &self,
        batch: &mut EventBatch,
        flush_type: FlushType,
    ) -> Result<(), StatsigErr> {
        let statsig_metadata = StatsigMetadata::get_with_log_event_extras(
            self.flush_interval.get_current_flush_interval_ms(),
            self.queue.batch_size,
            self.queue.max_pending_batches,
            flush_type.to_string(),
        );

        let result = self
            .logging_adapter
            .log_events(batch.get_log_event_request(statsig_metadata))
            .await;

        batch.attempts += 1;

        match result {
            Ok(true) => {
                self.ops_stats.log_event_request_success(batch.events.len());
                Ok(())
            }
            Ok(false) => Err(StatsigErr::LogEventError("Unknown Failure".into())),
            Err(StatsigErr::NetworkError(NetworkError::DisableNetworkOn(_))) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn prepare_event_queue_for_flush(&self, flush_type: FlushType) {
        self.try_add_non_exposed_checks_event();

        let dropped_events_count = match self.queue.reconcile_batching() {
            QueueResult::Success => return,
            QueueResult::LockFailure => {
                log_e!(TAG, "prepare_event_queue_for_flush lock failure");
                return;
            }
            QueueResult::DroppedEvents(dropped_events_count) => dropped_events_count,
        };

        if dropped_events_count > 0 {
            self.log_dropped_event_warning(dropped_events_count);

            self.ops_stats.log_batching_dropped_events(
                StatsigErr::LogEventError("Dropped events due to event queue limit".to_string()),
                dropped_events_count,
                &self.flush_interval,
                &self.queue,
                flush_type,
            );
        }
    }

    fn try_requeue_failed_batch(
        &self,
        error: &StatsigErr,
        batch: EventBatch,
        flush_type: FlushType,
    ) {
        let is_non_retryable = matches!(
            error,
            StatsigErr::NetworkError(NetworkError::RequestNotRetryable(_, _, _))
        );

        let is_max_retries = batch.attempts > EventLoggerConstants::max_log_event_retries();

        if is_non_retryable || is_max_retries {
            self.drop_events_for_failure(error, batch, flush_type);
            return;
        }

        let dropped_events_count = match self.queue.requeue_batch(batch) {
            QueueResult::Success => return,
            QueueResult::DroppedEvents(dropped_events_count) => dropped_events_count,
            QueueResult::LockFailure => {
                log_e!(TAG, "try_requeue_failed_batch lock failure");
                return;
            }
        };

        if dropped_events_count == 0 {
            return;
        }

        self.log_dropped_event_warning(dropped_events_count);

        self.ops_stats.log_batching_dropped_events(
            StatsigErr::LogEventError(
                "Dropped events due to max pending event batches limit".to_string(),
            ),
            dropped_events_count,
            &self.flush_interval,
            &self.queue,
            flush_type,
        );
    }

    fn drop_events_for_failure(
        &self,
        error: &StatsigErr,
        batch: EventBatch,
        flush_type: FlushType,
    ) {
        let dropped_events_count = batch.events.len() as u64;

        let kind = match flush_type {
            FlushType::ScheduledMaxTime => "Scheduled (Max Time)",
            FlushType::ScheduledFullBatch => "Scheduled (Full Batch)",
            FlushType::Limit => "Limit",
            FlushType::Manual => "Manual",
            FlushType::Shutdown => "Shutdown",
        };

        log_w!(
            TAG,
            "{} flush failed after {} attempt(s). {} Event(s) will be dropped. {}",
            kind,
            batch.attempts,
            dropped_events_count,
            error
        );

        self.ops_stats
            .log_event_request_failure(dropped_events_count, flush_type);
    }

    fn try_add_non_exposed_checks_event(&self) {
        let mut non_exposed_checks = write_lock_or_noop!(TAG, self.non_exposed_checks);
        if non_exposed_checks.is_empty() {
            return;
        }

        let checks = std::mem::take(&mut *non_exposed_checks);
        self.queue.add(QueuedEvent::Passthrough(
            StatsigEventInternal::new_non_exposed_checks_event(checks),
        ));
    }

    fn log_dropped_event_warning(&self, dropped_events_count: u64) {
        let approximate_pending_events_count = self.queue.approximate_pending_events_count();
        log_w!(
            TAG,
            "Too many events. Dropped {}. Approx pending events {}. Max pending batches {}. Max queue size {}",
            dropped_events_count,
            approximate_pending_events_count,
            self.queue.max_pending_batches,
            self.queue.batch_size
        );
    }
}
