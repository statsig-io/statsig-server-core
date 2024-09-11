use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::background_task::{BackgroundTask, BackgroundTaskRunner};
use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_exposure::StatsigExposure;
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_e, StatsigOptions};
use chrono::Utc;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Handle;
use tokio::time::Duration;
use crate::event_logging_adapter::EventLoggingAdapter;

const DEFAULT_FLUSH_INTERVAL_MS: u32 = 60_000;
const DEFAULT_QUEUE_SIZE: u32 = 500;
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
    max_queue_size: usize,
    background_flush_runner: BackgroundTaskRunner,
    previous_exposure_info: Arc<Mutex<PreviousExposureInfo>>,
    runtime_handle: Handle,
}

impl EventLogger {
    pub fn new(
        event_logging_adapter: Arc<dyn EventLoggingAdapter>,
        options: &StatsigOptions,
        runtime_handle: &Handle,
    ) -> Self {
        let flush_interval_ms = options
            .event_logging_flush_interval_ms
            .unwrap_or(DEFAULT_FLUSH_INTERVAL_MS);

        let max_queue_size = options
            .event_logging_max_queue_size
            .unwrap_or(DEFAULT_QUEUE_SIZE);

        let previous_exposure_info = Arc::new(Mutex::new(PreviousExposureInfo {
            exposures: HashSet::new(),
            last_reset: Utc::now().timestamp_millis() as u64,
        }));

        Self {
            event_logging_adapter,
            event_queue: Arc::new(RwLock::new(vec![])),
            max_queue_size: max_queue_size as usize,
            background_flush_runner: BackgroundTaskRunner::new(flush_interval_ms, runtime_handle),
            previous_exposure_info,
            runtime_handle: runtime_handle.clone(),
        }
    }

    pub fn start_background_task(self: Arc<Self>) {
        let weak_inst = Arc::downgrade(&self);
        if let Err(e) = self.background_flush_runner.start(weak_inst) {
            log_e!("Failed to start background event log flushing. {}", e);
        }
    }

    pub fn enqueue(&self, payload: QueuedEventPayload) {
        let mut should_flush = false;
        if let Ok(mut mut_events) = self.event_queue.write() {
            mut_events.push(payload);
            let length = mut_events.len();
            should_flush = length >= self.max_queue_size;
        };

        if should_flush {
            self.flush();
        }
    }

    pub async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.flush_blocking().await;
        self.background_flush_runner.shutdown(timeout).await?;
        Ok(())
    }

    pub fn flush(&self) {
        let queue = self.event_queue.clone();
        let adapter = self.event_logging_adapter.clone();
        let prev_expos = self.previous_exposure_info.clone();

        self.runtime_handle.spawn(async move {
            Self::flush_impl(adapter, queue, prev_expos).await;
        });
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

        let event_count = processed_events.len();

        let payload = HashMap::from([
            ("events".to_string(), json!(processed_events)),
            ("statsigMetadata".to_string(), json!(StatsigMetadata::new())),
        ]);

        let _ = event_logging_adapter.log_events(payload, event_count).await;
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

impl BackgroundTask for EventLogger {
    fn run(&self) -> Pin<Box<dyn Future<Output=()> + Send>> {
        let queue = self.event_queue.clone();
        let adapter = self.event_logging_adapter.clone();
        let prev_expos = self.previous_exposure_info.clone();
        Box::pin(async move {
            Self::flush_impl(adapter, queue, prev_expos).await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statsig_user_internal::StatsigUserInternal;
    use crate::StatsigUser;
    use crate::evaluation::evaluation_details::EvaluationDetails;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::atomic::{AtomicU64, Ordering};
    use crate::event_logging::statsig_event::StatsigEvent;

    fn enqueue_single(logger: &EventLogger, user_id: &str, event_name: &str) {
        let user_internal =
            StatsigUserInternal::new(&StatsigUser::with_user_id(user_id.to_string()), &None);

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
        let adapter = Arc::new(MockAdapter::new());
        let mut opts = StatsigOptions::new();
        opts.event_logging_max_queue_size = Some(1);

        let logger = EventLogger::new(adapter.clone(), &opts, &Handle::try_current().unwrap());

        for i in 1..10 {
            enqueue_single(&logger, format!("user_{}", i).as_str(), "my_event");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_shutdown_flush() {
        let adapter = Arc::new(MockAdapter::new());
        let logger = Arc::new(EventLogger::new(
            adapter.clone(),
            &StatsigOptions::new(),
            &Handle::try_current().unwrap(),
        ));
        logger.clone().start_background_task();

        enqueue_single(&logger, "a_user", "my_event");

        logger.shutdown(Duration::from_millis(100)).await.unwrap();

        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_bg_flushing() {
        let adapter = Arc::new(MockAdapter::new());
        let mut opts = StatsigOptions::new();
        opts.event_logging_flush_interval_ms = Some(1);

        let logger = Arc::new(EventLogger::new(
            adapter.clone(),
            &opts,
            &Handle::try_current().unwrap(),
        ));
        logger.clone().start_background_task();
        enqueue_single(&logger, "a_user", "my_event");

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(adapter.log_events_called_times.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dedupe() {
        let adapter = Arc::new(MockAdapter::new());
        let logger = Arc::new(EventLogger::new(
            adapter.clone(),
            &StatsigOptions::new(),
            &Handle::try_current().unwrap(),
        ));
        logger.clone().start_background_task();

        enqueue_single(&logger, "a_user", "my_custom");
        enqueue_single(&logger, "a_user", "my_custom");

        let user_internal =
            StatsigUserInternal::new(&StatsigUser::with_user_id("a_user".to_string()), &None);
        let eval_details = EvaluationDetails::unrecognized_no_data();
        logger.enqueue(QueuedEventPayload::GateExposure(GateExposure {
            user: user_internal.clone(),
            gate_name: "a_gate".to_string(),
            value: false,
            rule_id: None,
            secondary_exposures: None,
            evaluation_details: eval_details.clone(),
        }));
        logger.enqueue(QueuedEventPayload::GateExposure(GateExposure {
            user: user_internal,
            gate_name: "a_gate".to_string(),
            value: false,
            rule_id: None,
            secondary_exposures: None,
            evaluation_details: eval_details,
        }));

        logger.shutdown(Duration::from_millis(100)).await.unwrap();

        assert_eq!(adapter.log_event_count.load(Ordering::SeqCst), 3);
    }

    struct MockAdapter {
        pub bind_called_times: AtomicU64,
        pub log_events_called_times: AtomicU64,
        pub log_event_count: AtomicU64,
    }

    impl MockAdapter {
        fn new() -> Self {
            Self {
                bind_called_times: AtomicU64::new(0),
                log_events_called_times: AtomicU64::new(0),
                log_event_count: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl EventLoggingAdapter for MockAdapter {
        fn bind(&self, _sdk_key: &str, _option: &StatsigOptions) {
            self.bind_called_times.fetch_add(1, Ordering::SeqCst);
        }

        async fn log_events(
            &self,
            _payload: HashMap<String, Value>,
            event_count: usize,
        ) -> Result<bool, String> {
            self.log_events_called_times.fetch_add(1, Ordering::SeqCst);
            self.log_event_count
                .fetch_add(event_count as u64, Ordering::SeqCst);
            Ok(true)
        }
    }
}
