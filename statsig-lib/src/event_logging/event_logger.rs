use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_e, log_w, LogEventPayload, LogEventRequest, StatsigOptions, StatsigRuntime,
};
use chrono::Utc;
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::time::{sleep, Duration};

const DEFAULT_FLUSH_INTERVAL_MS: u32 = 60_000;
const DEFAULT_QUEUE_SIZE: u32 = 500;
const DEFAULT_DISABLE_ALL_LOGGING: bool = false;
const DEDUPE_WINDOW_DURATION_MS: u64 = 60_000;
const DEDUPE_MAX_KEYS: usize = 100000;

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

pub struct EventLogger {
    event_logging_adapter: Arc<dyn EventLoggingAdapter>,
    event_queue: Arc<RwLock<Vec<QueuedEventPayload>>>,
    flush_interval_ms: u32,
    disable_all_logging: bool,
    max_queue_size: usize,
    previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
    is_limit_flushing: Arc<AtomicBool>,
    statsig_runtime: Arc<StatsigRuntime>,
}

impl EventLogger {
    pub fn new(
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
        }
    }

    pub fn start_background_task(self: Arc<Self>, statsig_runtime: &Arc<StatsigRuntime>) {
        let weak_inst = Arc::downgrade(&self);
        log_d!("Starting event logger background flush");
        statsig_runtime.spawn("event_logger_bg_flush", move |shutdown_notify| async move {
            log_d!("BG flush loop begin");

            loop {
                let strong_self = match weak_inst.upgrade() {
                    Some(strong_self) => strong_self,
                    None => {
                        log_w!("failed to upgrade weak instance");
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
            return;
        }
        let mut should_flush = false;
        if let Ok(mut mut_events) = self.event_queue.write() {
            mut_events.push(payload);
            let length = mut_events.len();
            should_flush = length >= self.max_queue_size;
        };

        if should_flush && !self.is_limit_flushing.load(Ordering::Relaxed) {
            log_d!("Flush limit reached. Flushing...");
            self.is_limit_flushing.store(true, Ordering::Relaxed);
            self.flush();
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

        self.statsig_runtime.spawn(
            "event_logger_flush_and_forget",
            |_shutdown_notify| async move {
                is_limit_flushing.store(false, Ordering::Relaxed);
                Self::flush_impl(adapter, queue, prev_expos).await;
            },
        );
    }

    pub async fn flush_blocking(&self) {
        let queue = self.event_queue.clone();
        let adapter = self.event_logging_adapter.clone();
        let prev_expos = self.previous_exposure_info.clone();
        Self::flush_impl(adapter, queue, prev_expos).await;
    }

    async fn flush_impl(
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        queue: Arc<RwLock<Vec<QueuedEventPayload>>>,
        previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
    ) {
        log_d!("Attempting to flush events");
        let count = match queue.read().ok() {
            Some(e) => e.len(),
            _ => return,
        };

        let mut opt_local_events = None;
        if count != 0 {
            if let Ok(mut lock) = queue.write() {
                opt_local_events = Some(std::mem::take(&mut *lock));
                drop(lock);
            }
        }

        let processed_events: Vec<StatsigEventInternal> = match opt_local_events {
            Some(local_events) => local_events
                .into_iter()
                .filter_map(|p| validate_queued_event_payload(p, &previous_exposure_info))
                .collect(),
            None => return,
        };

        if processed_events.is_empty() {
            return;
        }

        let chunks = processed_events.chunks(1000);

        let futures: Vec<_> = chunks
            .map(|chunk| {
                let event_count = chunk.len() as u64;
                let request = LogEventRequest {
                    payload: LogEventPayload {
                        events: json!(processed_events),
                        statsig_metadata: StatsigMetadata::get_as_json(),
                    },
                    event_count,
                };

                log_d!("Preparing to flush {} events", event_count);
                event_logging_adapter.log_events(request)
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        for result in results {
            if let Err(e) = result {
                log_w!("Failed to flush events: {:?}", e);
            }
        }
    }
}

fn validate_queued_event_payload(
    payload: QueuedEventPayload,
    previous_exposure_info: &Mutex<PreviousExposureInfo>,
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
    previous_exposure_info: &Mutex<PreviousExposureInfo>,
) -> Option<StatsigEventInternal> {
    let mut previous_exposure_info = match previous_exposure_info.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log_e!("Failed to lock mutex: {}", e);
            return Some(exposure.to_internal_event());
        }
    };

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
        let logger = Arc::new(EventLogger::new(adapter.clone(), &opts, &statsig_rt));

        for i in 1..10 {
            enqueue_single(&logger, format!("user_{}", i).as_str(), "my_event");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_shutdown_flush() {
        initialize_simple_output_logger(&Some(LogLevel::Debug));

        let adapter = Arc::new(MockAdapter::new());
        let statsig_rt = StatsigRuntime::get_runtime();
        let logger = Arc::new(EventLogger::new(
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
        let logger = Arc::new(EventLogger::new(adapter.clone(), &opts, &statsig_rt));
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
        }));
        logger.enqueue(QueuedEventPayload::GateExposure(GateExposure {
            user: user_internal,
            gate_name: "a_gate".to_string(),
            value: false,
            rule_id: None,
            secondary_exposures: None,
            evaluation_details: eval_details,
            version: None,
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
    }
}
