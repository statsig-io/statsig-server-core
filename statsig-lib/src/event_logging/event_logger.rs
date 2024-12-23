use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::log_event_payload::{LogEventPayload, LogEventRequest};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_d, log_e, log_error_to_statsig_and_console, log_w, StatsigOptions, StatsigRuntime};
use chrono::Utc;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::time::{sleep, Duration};

use super::statsig_event::StatsigEvent;

const DEFAULT_FLUSH_INTERVAL_MS: u32 = 60_000;
const DEFAULT_QUEUE_SIZE: u32 = 500;
const DEFAULT_DISABLE_ALL_LOGGING: bool = false;
const DEDUPE_WINDOW_DURATION_MS: u64 = 60_000;
const DEDUPE_MAX_KEYS: usize = 100000;
const NON_EXPOSED_CHECKS_EVENT: &str = "statsig::non_exposed_checks";

const FLUSH_AND_FORGET_BG_TAG: &str = "event_logger_flush_and_forget";

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
            QueuedEventPayload::CustomEvent(e) => write!(f, "CustomEvent: {}", e.event_data.event_name),
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
    flush_interval_ms: u32,
    disable_all_logging: bool,
    max_queue_size: usize,
    previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
    is_limit_flushing: Arc<AtomicBool>,
    statsig_runtime: Arc<StatsigRuntime>,
    non_exposed_checks: Arc<Mutex<HashMap<String, u64>>>,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl EventLogger {
    pub fn new(
        sdk_key: &str,
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        options: &StatsigOptions,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Self {
        let flush_interval_ms = options
            .event_logging_flush_interval_ms
            .unwrap_or(DEFAULT_FLUSH_INTERVAL_MS);

        let max_queue_size = options
            .event_logging_max_queue_size
            .unwrap_or(DEFAULT_QUEUE_SIZE);

        let disable_all_logging = options
            .disable_all_logging
            .unwrap_or(DEFAULT_DISABLE_ALL_LOGGING);

        let previous_exposure_info = Arc::new(Mutex::new(PreviousExposureInfo {
            exposures: HashSet::new(),
            last_reset: Utc::now().timestamp_millis() as u64,
        }));

        Self {
            event_logging_adapter,
            event_queue: Arc::new(RwLock::new(vec![])),
            flush_interval_ms,
            disable_all_logging,
            max_queue_size: max_queue_size as usize,
            previous_exposure_info,
            is_limit_flushing: Arc::new(AtomicBool::new(false)),
            statsig_runtime: statsig_runtime.clone(),
            non_exposed_checks: Arc::new(Mutex::new(HashMap::new())),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
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
        statsig_runtime.spawn(TAG, move |shutdown_notify| async move {
            log_d!(TAG, "BG flush loop begin");

            loop {
                let strong_self = match weak_inst.upgrade() {
                    Some(strong_self) => strong_self,
                    None => {
                        log_w!(TAG, "failed to upgrade weak instance");
                        break;
                    }
                };

                tokio::select! {
                    _ = sleep(Duration::from_millis(strong_self.flush_interval_ms as u64)) => {
                        strong_self.flush_blocking().await;
                    }
                    _ = shutdown_notify.notified() => {
                        break;
                    }
                }
            }
        });
    }

    pub fn enqueue(&self, payload: QueuedEventPayload) {
        if self.disable_all_logging {
            log_d!(TAG, "Did not enqueue {} because all logging is disabled", payload);
            return;
        }

        log_d!(TAG, "Enqueued {}", payload);

        let mut should_flush = false;
        if let Ok(mut mut_events) = self.event_queue.write() {
            mut_events.push(payload);
            let length = mut_events.len();
            should_flush = length >= self.max_queue_size;
        };

        if should_flush && !self.is_limit_flushing.load(Ordering::Relaxed) {
            log_d!(TAG, "Flush limit reached. Flushing...");
            self.is_limit_flushing.store(true, Ordering::Relaxed);
            self.flush();
        }
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
        self.flush_blocking().await;
        Ok(())
    }

    pub fn flush(&self) {
        let queue = self.event_queue.clone();
        let adapter = self.event_logging_adapter.clone();
        let prev_expos = self.previous_exposure_info.clone();
        let is_limit_flushing = self.is_limit_flushing.clone();
        let non_exposed_checks = self.non_exposed_checks.clone();
        let ops_stats = self.ops_stats.clone();

        self.statsig_runtime.spawn(
            FLUSH_AND_FORGET_BG_TAG,
            |_shutdown_notify| async move {
                is_limit_flushing.store(false, Ordering::Relaxed);
                Self::flush_impl(adapter, queue, prev_expos, non_exposed_checks, ops_stats).await;
            },
        );
    }

    pub async fn flush_blocking(&self) {
        let queue = self.event_queue.clone();
        let adapter = self.event_logging_adapter.clone();
        let prev_expos = self.previous_exposure_info.clone();
        let non_exposed_checks = self.non_exposed_checks.clone();
        let ops_stats = self.ops_stats.clone();

        Self::flush_impl(adapter, queue, prev_expos, non_exposed_checks, ops_stats).await;
        self.statsig_runtime.await_tasks_with_tag(FLUSH_AND_FORGET_BG_TAG).await;
    }

    async fn flush_impl(
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        queue: Arc<RwLock<Vec<QueuedEventPayload>>>,
        previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
        non_exposed_checks: Arc<Mutex<HashMap<String, u64>>>,
        ops_stats: Arc<OpsStatsForInstance>,
    ) {
        log_d!(TAG, "Attempting to flush events");

        append_non_exposed_event_and_reset(&queue, &non_exposed_checks);

        let count = match queue.read().ok() {
            Some(e) => e.len(),
            _ => return,
        };

        if count == 0 {
            return;
        }

        let payloads = match queue.write() {
            Ok(mut lock) => std::mem::take(&mut *lock),
            _ => {
                log_error_to_statsig_and_console!(ops_stats, TAG, "Failed to lock event queue");
                return;
            }
        };

        let validated_chunks = validate_and_chunk_events(payloads, previous_exposure_info).await;

        if validated_chunks.is_empty() {
            return;
        }

        let tasks = validated_chunks.iter().map(|chunk| {
            let event_count = chunk.len() as u64;
            let request = LogEventRequest {
                payload: LogEventPayload {
                    events: json!(chunk),
                    statsig_metadata: StatsigMetadata::get_as_json(),
                },
                event_count,
            };
            event_logging_adapter.log_events(request)
        });

        let results = futures::future::join_all(tasks).await;
        for result in results {
            if let Err(e) = result {
                ops_stats.log_error(ErrorBoundaryEvent {
                    tag: TAG.to_string(),
                    exception: format!("Failed to flush events: {}", e),
                });
                log_w!(TAG, "Failed to flush events: {:?}", e);
            }
        }
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

async fn validate_and_chunk_events(
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

        if chunk.len() >= 1000 {
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
    use crate::statsig_user_internal::StatsigUserInternal;
    use crate::StatsigUser;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn enqueue_single(logger: &EventLogger, user_id: &str, event_name: &str) {
        let user_internal =
            StatsigUserInternal::new(&StatsigUser::with_user_id(user_id.to_string()), None);

        let event = StatsigEventInternal::new(
            user_internal,
            StatsigEvent {
                event_name: event_name.to_string(),
                value: None,
                metadata: None,
            },
            None,
        );

        logger.enqueue(QueuedEventPayload::CustomEvent(event));
    }

    #[tokio::test]
    async fn test_limit_flushing() {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let adapter = Arc::new(MockAdapter::new());
        let mut opts = StatsigOptions::new();
        opts.event_logging_max_queue_size = Some(1);

        let statsig_rt = StatsigRuntime::get_runtime();
        let logger = Arc::new(EventLogger::new(
            "secret-key",
            adapter.clone(),
            &opts,
            &statsig_rt,
        ));

        for i in 1..10 {
            enqueue_single(&logger, format!("user_{}", i).as_str(), "my_event");
        }

        sleep(Duration::from_millis(100)).await;
        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
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
            StatsigUserInternal::new(&StatsigUser::with_user_id("a_user".to_string()), None);
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
