use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::global_configs::GlobalConfigs;
use crate::log_event_payload::{LogEventPayload, LogEventRequest};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, log_w, StatsigOptions, StatsigRuntime,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

use super::event_logger_dynamic_defaults::EventLoggerDynamicDefaults;
use super::statsig_event::StatsigEvent;

const MAX_FLUSH_INTERVAL_MS: u64 = 60_000;
const DEFAULT_QUEUE_SIZE: u32 = 1000;
const DEFAULT_PENDING_BATCH_QUEUE_SIZE: u32 = 10;
const DEFAULT_DISABLE_ALL_LOGGING: bool = false;
const DEDUPE_WINDOW_DURATION_MS: u64 = 60_000;
const DEDUPE_MAX_KEYS: usize = 100_000;
const MAX_EVENT_RETRY: u32 = 5;
const NON_EXPOSED_CHECKS_EVENT: &str = "statsig::non_exposed_checks";

const BATCH_AND_FORGET_BG_TAG: &str = "event_logger_batch_and_forget";

struct PreviousExposureInfo {
    exposures: HashSet<String>,
    last_reset: u64,
}

pub enum QueuedEventPayload {
    CustomEvent(StatsigEventInternal),
    GateExposure(GateExposure),
    ConfigExposure(ConfigExposure),
    LayerExposure(LayerExposure),
}

impl Display for QueuedEventPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuedEventPayload::CustomEvent(e) => {
                write!(f, "CustomEvent: {}", e.event_data.event_name)
            }
            QueuedEventPayload::GateExposure(e) => write!(f, "GateExposure: {}", e.gate_name),
            QueuedEventPayload::ConfigExposure(e) => write!(f, "ConfigExposure: {}", e.config_name),
            QueuedEventPayload::LayerExposure(e) => write!(f, "LayerExposure: {}", e.layer_name),
        }
    }
}

const TAG: &str = stringify!(EventLogger);
pub struct EventLogger {
    event_logging_adapter: Arc<dyn EventLoggingAdapter>,
    event_queue: Arc<RwLock<Vec<QueuedEventPayload>>>,
    pending_batch_queue: Arc<RwLock<VecDeque<LogEventRequest>>>,
    disable_all_logging: bool,
    max_queue_size: usize,
    max_pending_events_queue_size: usize,
    flush_interval_ms: AtomicU64,
    dropped_event_count: AtomicU64,
    previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
    is_limit_batching: AtomicBool,
    statsig_runtime: Arc<StatsigRuntime>,
    non_exposed_checks: Arc<Mutex<HashMap<String, u64>>>,
    ops_stats: Arc<OpsStatsForInstance>,
    global_configs: Arc<GlobalConfigs>,
    shutdown_notify: Arc<Notify>,
    defaults: EventLoggerDynamicDefaults,
}

impl EventLogger {
    pub fn new(
        sdk_key: &str,
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        options: &StatsigOptions,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Self {
        let defaults = EventLoggerDynamicDefaults::new();

        let max_queue_size = options
            .event_logging_max_queue_size
            .unwrap_or(DEFAULT_QUEUE_SIZE);

        let disable_all_logging = options
            .disable_all_logging
            .unwrap_or(DEFAULT_DISABLE_ALL_LOGGING);

        let max_pending_events_queue_size = options
            .event_logging_max_pending_batch_queue_size
            .unwrap_or(DEFAULT_PENDING_BATCH_QUEUE_SIZE);

        let previous_exposure_info = Arc::new(Mutex::new(PreviousExposureInfo {
            exposures: HashSet::new(),
            last_reset: Utc::now().timestamp_millis() as u64,
        }));

        let global_configs = GlobalConfigs::get_instance(sdk_key);

        let override_interval_ms =
            Self::get_override_interval_ms(&global_configs, defaults.flush_interval_ms);

        let flushing_interval_ms = override_interval_ms.unwrap_or(defaults.flush_interval_ms);

        Self {
            event_logging_adapter,
            event_queue: Arc::new(RwLock::new(vec![])),
            pending_batch_queue: Arc::new(RwLock::new(VecDeque::new())),
            flush_interval_ms: AtomicU64::new(flushing_interval_ms),
            disable_all_logging,
            max_queue_size: max_queue_size as usize,
            max_pending_events_queue_size: max_pending_events_queue_size as usize,
            dropped_event_count: AtomicU64::new(0),
            previous_exposure_info,
            is_limit_batching: AtomicBool::new(false),
            statsig_runtime: statsig_runtime.clone(),
            non_exposed_checks: Arc::new(Mutex::new(HashMap::new())),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            global_configs,
            shutdown_notify: Arc::new(Notify::new()),
            defaults,
        }
    }

    pub fn start_background_task(self: Arc<Self>, statsig_runtime: &Arc<StatsigRuntime>) {
        if !self
            .event_logging_adapter
            .should_schedule_background_flush()
        {
            return;
        }

        let weak_inst = Arc::downgrade(&self);
        log_d!(TAG, "Starting event logger background flush");

        statsig_runtime.spawn(TAG, move |rt_shutdown_notify| async move {
            log_d!(TAG, "BG flush loop begin");
            let mut last_batch_time = Instant::now();

            loop {
                let strong_self = if let Some(strong_self) = weak_inst.upgrade() { strong_self } else {
                    log_w!(TAG, "failed to upgrade weak instance");
                    break;
                };

                let flush_interval = self.flush_interval_ms.load(Ordering::Relaxed);
                tokio::select! {
                    () = sleep(Duration::from_millis(flush_interval)) => {
                        Self::background_flush_actions(strong_self, &mut last_batch_time, flush_interval).await;
                    }
                    () = rt_shutdown_notify.notified() => {
                        log_d!(TAG, "Runtime shutdown. Shutting down event logger background flush");
                        break;
                    }
                    () = strong_self.shutdown_notify.notified() => {
                        log_d!(TAG, "Shutting down event logger background flush");
                        break;
                    }
                }
            }
        });
    }

    pub fn enqueue(self: &Arc<Self>, payload: QueuedEventPayload) {
        if self.disable_all_logging {
            log_d!(
                TAG,
                "Did not enqueue {} because all logging is disabled",
                payload
            );
            return;
        }

        let mut should_batch = false;
        if let Ok(mut mut_events) = self.event_queue.write() {
            mut_events.push(payload);
            let length = mut_events.len();
            should_batch = length >= self.max_queue_size;
        };

        if should_batch {
            self.try_batch_and_prepare_events();
        }
    }

    fn try_batch_and_prepare_events(self: &Arc<Self>) {
        let was_not_batching = !self.is_limit_batching.swap(true, Ordering::Relaxed);
        if !was_not_batching {
            return;
        }

        let self_clone = Arc::clone(self);
        self.statsig_runtime
            .spawn(BATCH_AND_FORGET_BG_TAG, |_shutdown_notify| async move {
                tokio::select! {
                    _ = self_clone.shutdown_notify.notified() => {
                        log_d!(
                            TAG,
                            "Shutdown signaled. Canceling event batching."
                        );
                    }
                    _ = async {
                        log_d!(TAG,"Event queue limit hit. Batching and preparing events...");
                        self_clone.batch_and_prepare_events();
                    } => {}
                }
            });
    }

    pub fn increment_non_exposure_checks_count(&self, name: String) {
        match self.non_exposed_checks.lock() {
            Ok(mut map) => {
                *map.entry(name).or_insert(0) += 1;
            }
            Err(e) => {
                log_w!(TAG, "Failed to increment non exposure checks' count {}", e);
            }
        }
    }

    pub async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.flush_all_blocking().await;
        self.shutdown_notify.notify_one();
        Ok(())
    }

    pub async fn flush_blocking(&self, flush_all: bool) {
        let adapter = self.event_logging_adapter.clone();
        let global_configs = self.global_configs.clone();
        let pending_event_batches = self.pending_batch_queue.clone();

        Self::flush_impl(
            adapter,
            pending_event_batches,
            flush_all,
            &self.flush_interval_ms,
            global_configs,
            &self.defaults,
        )
        .await;
    }

    pub async fn flush_all_blocking(&self) {
        self.batch_and_prepare_events();

        self.statsig_runtime
            .await_tasks_with_tag(BATCH_AND_FORGET_BG_TAG)
            .await;

        self.flush_blocking(true).await;
    }

    fn batch_and_prepare_events(&self) {
        append_non_exposed_event_and_reset(&self.event_queue, &self.non_exposed_checks);

        let count = match self.event_queue.read().ok() {
            Some(e) => e.len(),
            _ => return,
        };

        if count == 0 {
            return;
        }

        log_d!(TAG, "Batching and preparing {} events", count);

        let payloads = if let Ok(mut lock) = self.event_queue.write() {
            std::mem::take(&mut *lock)
        } else {
            log_error_to_statsig_and_console!(self.ops_stats, TAG, "Failed to lock event queue");
            return;
        };

        let limit_batching = self.is_limit_batching.swap(false, Ordering::Relaxed);

        let validated_chunks = validate_and_chunk_events(
            self.max_queue_size,
            payloads,
            self.previous_exposure_info.clone(),
        );

        if validated_chunks.is_empty() {
            return;
        }

        let mut pending_batches = match self.pending_batch_queue.write() {
            Ok(lock) => lock,
            Err(_) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats,
                    TAG,
                    "Failed to lock pending event batches"
                );
                return;
            }
        };

        for chunk in &validated_chunks {
            let event_count = chunk.len() as u64;
            let mut statsig_metadata = StatsigMetadata::get_as_json();
            if let Value::Object(ref mut obj) = statsig_metadata {
                obj.insert("isLimitBatch".to_string(), json!(limit_batching));
            }
            let pending_batch = LogEventRequest {
                payload: LogEventPayload {
                    events: json!(chunk),
                    statsig_metadata,
                },
                event_count,
                retries: 0,
            };

            if pending_batches.len() < self.max_pending_events_queue_size {
                pending_batches.push_back(pending_batch);
            } else {
                log_d!(
                    TAG,
                    "Dropping event batch because pending batch queue is full: event_count={}",
                    event_count
                );
                self.dropped_event_count
                    .fetch_add(event_count, Ordering::Relaxed);
            }
        }
    }

    fn success_backoff(flushing_interval_ms: &AtomicU64, min_flushing_interval_ms: u64) {
        let current_interval = flushing_interval_ms.load(Ordering::Relaxed);
        let new_interval = current_interval / 2;
        let clamped_interval = new_interval.max(min_flushing_interval_ms);
        flushing_interval_ms.store(clamped_interval, Ordering::Relaxed);
    }

    fn failure_backoff(flushing_interval_ms: &AtomicU64) {
        let current_interval = flushing_interval_ms.load(Ordering::Relaxed);
        let new_interval = current_interval * 2;
        let clamped_interval = new_interval.min(MAX_FLUSH_INTERVAL_MS);
        flushing_interval_ms.store(clamped_interval, Ordering::Relaxed);
    }

    fn get_override_interval_ms(
        global_configs: &Arc<GlobalConfigs>,
        default_flushing_interval: u64,
    ) -> Option<u64> {
        global_configs
            .get_sdk_config_value("event_logging_interval_seconds")
            .and_then(|v| v.float_value)
            .map(|rate| (rate * 1000.0) as u64)
            .filter(|&interval| interval != default_flushing_interval)
    }

    fn get_batches_to_process(
        pending_batches: &Arc<RwLock<VecDeque<LogEventRequest>>>,
        flush_all: bool,
    ) -> VecDeque<LogEventRequest> {
        let mut batches_guard = match pending_batches.write() {
            Ok(guard) => guard,
            Err(_) => {
                log_e!(TAG, "Failed to acquire write lock on pending_batches");
                return VecDeque::new();
            }
        };

        if flush_all {
            std::mem::take(&mut *batches_guard)
        } else {
            let mut single_batch = VecDeque::new();
            if let Some(batch) = batches_guard.pop_front() {
                single_batch.push_back(batch);
            }
            single_batch
        }
    }

    async fn background_flush_actions(
        strong_self: Arc<EventLogger>,
        last_batch_time: &mut Instant,
        curr_flush_interval: u64,
    ) {
        let now = Instant::now();
        strong_self.flush_blocking(false).await;

        if now.duration_since(*last_batch_time)
            >= Duration::from_millis(strong_self.defaults.batching_interval)
        {
            strong_self.batch_and_prepare_events();
            *last_batch_time = now;

            let dropped_count = strong_self.dropped_event_count.swap(0, Ordering::Relaxed);
            if dropped_count > 0 {
                log_w!(
                                    TAG,
                                    "Dropped {} events due to events input higher than event flushing QPS. \
                                     To learn more about event handling, visit https://docs.statsig.com/debugging#maximizing-event-throughput",
                                    dropped_count
                                );

                strong_self.ops_stats.log_error(ErrorBoundaryEvent {
                    tag: "statsig::log_event_dropped_event_count".to_string(),
                    exception: "EventsDropped".to_string(),
                    extra: Some(HashMap::from([
                        ("eventCount".to_string(), dropped_count.to_string()),
                        (
                            "loggingInterval".to_string(),
                            curr_flush_interval.to_string(),
                        ),
                    ])),
                });
            }
        }
    }

    async fn create_flush_tasks<'a>(
        batches_to_process: &'a VecDeque<LogEventRequest>,
        event_logging_adapter: &'a Arc<dyn EventLoggingAdapter>,
        global_configs: &'a Arc<GlobalConfigs>,
        defaults: &'a EventLoggerDynamicDefaults,
        flushing_interval_ms: &'a AtomicU64,
    ) -> Vec<impl Future<Output = Option<(usize, StatsigErr)>> + 'a> {
        batches_to_process
            .iter()
            .enumerate()
            .map(|(idx, batch)| {
                let mut batch_clone = batch.clone();

                let adapter = Arc::clone(event_logging_adapter);
                let override_interval_ms =
                    Self::get_override_interval_ms(global_configs, defaults.flush_interval_ms);
                let current_interval_ms = flushing_interval_ms.load(Ordering::Relaxed);

                if let Value::Object(ref mut obj) = batch_clone.payload.statsig_metadata {
                    obj.insert("flushingIntervalMs".to_string(), json!(current_interval_ms));
                }

                async move {
                    match adapter.log_events(batch_clone).await {
                        Ok(_) => {
                            if override_interval_ms.is_none() {
                                Self::success_backoff(
                                    flushing_interval_ms,
                                    defaults.min_flush_interval_ms,
                                );
                            }
                            None
                        }
                        Err(e) => {
                            log_w!(TAG, "Failed to flush events: {:?}", e);
                            if override_interval_ms.is_none() {
                                Self::failure_backoff(flushing_interval_ms);
                            }
                            Some((idx, e))
                        }
                    }
                }
            })
            .collect()
    }

    async fn flush_impl(
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        pending_batches: Arc<RwLock<VecDeque<LogEventRequest>>>,
        flush_all: bool,
        flushing_interval_ms: &AtomicU64,
        global_configs: Arc<GlobalConfigs>,
        defaults: &EventLoggerDynamicDefaults,
    ) {
        let batches_to_process = Self::get_batches_to_process(&pending_batches, flush_all);
        if batches_to_process.is_empty() {
            return;
        }

        log_d!(TAG, "Attempting to flush events");

        let tasks = Self::create_flush_tasks(
            &batches_to_process,
            &event_logging_adapter,
            &global_configs,
            defaults,
            flushing_interval_ms,
        )
        .await;

        let results = futures::future::join_all(tasks).await;

        let mut queue_lock = match pending_batches.write() {
            Ok(lock) => lock,
            Err(e) => {
                log_w!(
                    TAG,
                    "Failed to acquire write lock on pending_batch_queue: {}",
                    e
                );
                return;
            }
        };

        for (idx, _) in results.into_iter().flatten() {
            let mut failed_batch = batches_to_process[idx].clone();

            failed_batch.retries += 1;

            if failed_batch.retries > MAX_EVENT_RETRY {
                log_w!(
                    TAG,
                    "Dropping event batch after {} retries: event_count={}",
                    MAX_EVENT_RETRY,
                    failed_batch.event_count
                );
            } else {
                queue_lock.push_back(failed_batch);
            }
        }
        // TODO: check status code to know if its retryable and if not send or log event failure
    }
}

fn append_non_exposed_event_and_reset(
    queue: &Arc<RwLock<Vec<QueuedEventPayload>>>,
    non_exposed_checks: &Arc<Mutex<HashMap<String, u64>>>,
) {
    let mut map = match non_exposed_checks.lock() {
        Ok(lock) => lock,
        Err(e) => {
            log_d!(TAG, "Failed to acquire lock on non exposed checks: {}", e);
            return;
        }
    };

    if map.is_empty() {
        return;
    }

    let metadata = match serde_json::to_string(&*map) {
        Ok(json) => {
            let mut metadata_map = HashMap::new();
            metadata_map.insert("checks".to_string(), json);
            metadata_map
        }
        Err(e) => {
            log_d!(TAG, "Failed to serialize non_exposed_checks to JSON: {}", e);
            return;
        }
    };

    let event = StatsigEventInternal::new_non_exposed_checks_event(StatsigEvent {
        event_name: NON_EXPOSED_CHECKS_EVENT.to_string(),
        value: None,
        metadata: Some(metadata),
        statsig_metadata: None,
    });

    match queue.write() {
        Ok(mut lock) => {
            lock.push(QueuedEventPayload::CustomEvent(event));
            map.clear();
        }
        Err(_) => {
            log_d!(
                TAG,
                "Failed to acquire write lock when pushing non exposed check events"
            );
        }
    }
}

fn validate_and_chunk_events(
    max_chunk_size: usize,
    payloads: Vec<QueuedEventPayload>,
    previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
) -> Vec<Vec<StatsigEventInternal>> {
    let mut previous_info = match previous_exposure_info.lock() {
        Ok(lock) => lock,
        Err(e) => {
            log_e!(TAG, "Failed to lock previous exposure mutex: {}", e);
            return vec![];
        }
    };

    let mut valid_events = vec![];
    let mut chunk = vec![];

    for payload in payloads {
        if let Some(event) = validate_queued_event_payload(payload, &mut previous_info) {
            chunk.push(event);
        }

        if chunk.len() >= max_chunk_size {
            valid_events.push(chunk);
            chunk = vec![];
        }
    }

    if !chunk.is_empty() {
        valid_events.push(chunk);
    }

    valid_events
}

fn validate_queued_event_payload(
    payload: QueuedEventPayload,
    previous_exposure_info: &mut PreviousExposureInfo,
) -> Option<StatsigEventInternal> {
    match payload {
        QueuedEventPayload::CustomEvent(e) => Some(e),
        QueuedEventPayload::GateExposure(expo) => {
            validate_exposure_event(expo, previous_exposure_info)
        }
        QueuedEventPayload::ConfigExposure(expo) => {
            validate_exposure_event(expo, previous_exposure_info)
        }
        QueuedEventPayload::LayerExposure(expo) => {
            validate_exposure_event(expo, previous_exposure_info)
        }
    }
}

fn validate_exposure_event<T: StatsigExposure>(
    exposure: T,
    previous_exposure_info: &mut PreviousExposureInfo,
) -> Option<StatsigEventInternal> {
    let now = Utc::now().timestamp_millis() as u64;
    if now - previous_exposure_info.last_reset > DEDUPE_WINDOW_DURATION_MS
        || previous_exposure_info.exposures.len() > DEDUPE_MAX_KEYS
    {
        previous_exposure_info.exposures.clear();
        previous_exposure_info.last_reset = now;
    }

    let dedupe_key = exposure.make_dedupe_key();
    if previous_exposure_info.exposures.contains(&dedupe_key) {
        return None;
    }

    previous_exposure_info.exposures.insert(dedupe_key);
    Some(exposure.to_internal_event())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::evaluation_details::EvaluationDetails;
    use crate::event_logging::statsig_event::StatsigEvent;
    use crate::output_logger::{initialize_simple_output_logger, LogLevel};
    use crate::sampling_processor::SamplingDecision;
    use crate::statsig_user_internal::StatsigUserInternal;
    use crate::StatsigUser;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn enqueue_single(logger: &Arc<EventLogger>, user_id: &str, event_name: &str) {
        let user_internal =
            StatsigUserInternal::new(&StatsigUser::with_user_id(user_id.to_string()), None, None);

        let mut sampling_statsig_metadata: HashMap<String, Value> = HashMap::new();
        sampling_statsig_metadata.insert("samplingMode".into(), "on".into());
        sampling_statsig_metadata.insert("samplingRate".into(), 101.into());

        let event = StatsigEventInternal::new(
            user_internal,
            StatsigEvent {
                event_name: event_name.to_string(),
                value: None,
                metadata: None,
                statsig_metadata: Some(sampling_statsig_metadata),
            },
            None,
        );

        logger
            .clone()
            .enqueue(QueuedEventPayload::CustomEvent(event));
    }

    #[tokio::test]
    async fn test_shutdown_flush() {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let adapter = Arc::new(MockAdapter::new());
        let statsig_rt = StatsigRuntime::get_runtime();
        let logger = Arc::new(EventLogger::new(
            "secret-key",
            adapter.clone(),
            &StatsigOptions::new(),
            &statsig_rt,
        ));
        logger.clone().start_background_task(&statsig_rt);

        enqueue_single(&logger, "a_user", "my_event");

        logger.shutdown(Duration::from_millis(100)).await.unwrap();

        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_bg_flushing() {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let adapter = Arc::new(MockAdapter::new());
        let mut opts = StatsigOptions::new();
        opts.event_logging_flush_interval_ms = Some(1);

        let statsig_rt = StatsigRuntime::get_runtime();
        let logger = Arc::new(EventLogger::new(
            "secret-key",
            adapter.clone(),
            &opts,
            &statsig_rt,
        ));
        logger.clone().start_background_task(&statsig_rt);
        enqueue_single(&logger, "a_user", "my_event");

        sleep(Duration::from_millis(100)).await;

        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dedupe() {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let statsig_rt = StatsigRuntime::get_runtime();
        let adapter = Arc::new(MockAdapter::new());
        let logger = Arc::new(EventLogger::new(
            "secret-key",
            adapter.clone(),
            &StatsigOptions::new(),
            &statsig_rt,
        ));
        logger.clone().start_background_task(&statsig_rt);

        enqueue_single(&logger, "a_user", "my_custom");
        enqueue_single(&logger, "a_user", "my_custom");

        let user_internal =
            StatsigUserInternal::new(&StatsigUser::with_user_id("a_user".to_string()), None, None);
        let eval_details = EvaluationDetails::unrecognized_no_data();

        logger.enqueue(QueuedEventPayload::GateExposure(GateExposure {
            user: user_internal.clone(),
            gate_name: "a_gate".to_string(),
            value: false,
            rule_id: None,
            secondary_exposures: None,
            evaluation_details: eval_details.clone(),
            version: None,
            is_manual_exposure: false,
            sampling_details: SamplingDecision::default(),
            override_config_name: None,
        }));
        logger.enqueue(QueuedEventPayload::GateExposure(GateExposure {
            user: user_internal,
            gate_name: "a_gate".to_string(),
            value: false,
            rule_id: None,
            secondary_exposures: None,
            evaluation_details: eval_details,
            version: None,
            is_manual_exposure: false,
            sampling_details: SamplingDecision::default(),
            override_config_name: None,
        }));

        logger.shutdown(Duration::from_millis(100)).await.unwrap();

        assert_eq!(adapter.log_event_count.load(Ordering::SeqCst), 3);
    }

    struct MockAdapter {
        pub log_events_called_times: AtomicU64,
        pub log_event_count: AtomicU64,
    }

    impl MockAdapter {
        fn new() -> Self {
            Self {
                log_events_called_times: AtomicU64::new(0),
                log_event_count: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl EventLoggingAdapter for MockAdapter {
        async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
            self.log_events_called_times.fetch_add(1, Ordering::SeqCst);
            self.log_event_count
                .fetch_add(request.event_count, Ordering::SeqCst);
            Ok(true)
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        fn should_schedule_background_flush(&self) -> bool {
            true
        }
    }
}
