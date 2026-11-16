use super::{SpecsInfo, StatsigHttpSpecsAdapter};
use crate::networking::ResponseData;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, log_w, SpecAdapterConfig, SpecsAdapter,
    SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigOptions, StatsigRuntime,
};
use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use sigstat_grpc::statsig_grpc_client::StatsigGrpcClient;
use std::cmp;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Notify};
use tokio::time::{sleep, timeout};

// Todo make those configurable
const DEFAULT_BACKOFF_INTERVAL_MS: u64 = 3000;
const DEFAULT_BACKOFF_MULTIPLIER: u64 = 2;
const MAX_BACKOFF_INTERVAL_MS: u64 = 60 * 1000;
const RETRY_LIMIT: u64 = 10 * 24 * 60 * 60;
const FALL_BACK_TO_POLLING_THREASHOLD: u64 = 30; //Fallback after 30 minutes
struct StreamingRetryState {
    backoff_interval_ms: AtomicU64,
    retry_attempts: AtomicU64,
    is_retrying: AtomicBool,
}

struct ProcessLocalGrpcTask {
    pid: u32,
    handle_id: tokio::task::Id,
}

const TAG: &str = stringify!(StatsigGrpcSpecsAdapter);
const BG_TASK_TAG: &str = "grpc_streaming";

pub struct StatsigGrpcSpecsAdapter {
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
    shutdown_generation: Arc<AtomicU64>,
    initialization_tx: Arc<broadcast::Sender<Result<(), StatsigErr>>>,
    task_handle: ArcSwapOption<ProcessLocalGrpcTask>,
    grpc_client: StatsigGrpcClient,
    retry_state: StreamingRetryState,
    init_timeout: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
    // Set while initialize waits for the first streamed spec, so that message
    // closes the initialize network marker exactly once.
    awaiting_init_message: AtomicBool,
    // For fallback to poll job behavior
    http_specs_adapter: Arc<StatsigHttpSpecsAdapter>,
    cancel_poll_notify: Arc<Notify>,
}

#[async_trait]
impl SpecsAdapter for StatsigGrpcSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.awaiting_init_message.store(true, Ordering::SeqCst);
        self.add_diagnostics_marker(
            ContextType::Initialize,
            ActionType::Start,
            StepType::NetworkRequest,
            None,
        );
        // Subscribe before spawning so the first streamed result cannot be missed.
        let mut rx = self.initialization_tx.subscribe();
        let handle_id = match self
            .clone()
            .spawn_grpc_streaming_thread(statsig_runtime, self.ops_stats.clone())
            .await
        {
            Ok(handle_id) => handle_id,
            Err(e) => {
                self.end_init_network_marker_on_failure();
                return Err(e);
            }
        };

        self.set_task_handle(handle_id);
        let result = match timeout(self.init_timeout, rx.recv()).await {
            Ok(res) => match res {
                Ok(Ok(())) => Ok(()),
                Ok(Err(err)) => Err(StatsigErr::GrpcError(format!(
                    "Failed to initialize from streaming: {err}"
                ))),
                Err(_) => Err(StatsigErr::GrpcError("Failed to get a ".to_string())),
            },
            Err(_) => Err(StatsigErr::GrpcError(
                "Start Timeout to get a response".to_string(),
            )),
        };
        self.end_init_network_marker_on_failure();
        result
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        if self.task_handle.load_full().is_some_and(|task| {
            task.pid == std::process::id()
                && statsig_runtime.has_running_task(BG_TASK_TAG, &task.handle_id)
        }) {
            return Ok(());
        }

        let task_id = self
            .clone()
            .spawn_grpc_streaming_thread(statsig_runtime, self.ops_stats.clone())
            .await?;

        self.set_task_handle(task_id);

        Ok(())
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        self.http_specs_adapter.initialize(listener.clone());
        match self
            .listener
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut lock) => *lock = Some(listener),
            None => {
                log_error_to_statsig_and_console!(
                    self.ops_stats,
                    TAG,
                    StatsigErr::LockFailure("Failed to acquire write lock on listener".to_string())
                );
            }
        }
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.signal_shutdown();

        let handle_id = match self.task_handle.swap(None) {
            Some(task) if task.pid == std::process::id() => task.handle_id,
            None => {
                return Err(StatsigErr::ThreadFailure(
                    "No running task found".to_string(),
                ));
            }
            Some(_) => {
                return Err(StatsigErr::ThreadFailure(
                    "No running task found for this process".to_string(),
                ));
            }
        };

        if tokio::time::timeout(
            timeout,
            statsig_runtime.await_join_handle(BG_TASK_TAG, &handle_id),
        )
        .await
        .is_err()
        {
            return Err(StatsigErr::GrpcError(
                "Failed to gracefully shutdown StatsigGrpcSpecsAdapter.".to_string(),
            ));
        }

        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigGrpcSpecsAdapter).to_string()
    }
}

impl StatsigGrpcSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        config: &SpecAdapterConfig,
        options: Option<&StatsigOptions>,
    ) -> Self {
        let fallback_adapter = StatsigHttpSpecsAdapter::new(sdk_key, options, None);
        let (init_tx, _) = broadcast::channel(1);
        Self {
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
            shutdown_generation: Arc::new(AtomicU64::new(0)),
            task_handle: ArcSwapOption::empty(),
            grpc_client: StatsigGrpcClient::new(
                sdk_key,
                &config.specs_url.clone().unwrap_or("INVALID".to_owned()),
                config.authentication_mode.clone(),
                config.ca_cert_path.clone(),
                config.client_cert_path.clone(),
                config.client_key_path.clone(),
                config.domain_name.clone(),
            ),
            initialization_tx: Arc::new(init_tx),
            retry_state: StreamingRetryState {
                backoff_interval_ms: DEFAULT_BACKOFF_INTERVAL_MS.into(),
                retry_attempts: 0.into(),
                is_retrying: false.into(),
            },
            init_timeout: Duration::from_millis(if config.init_timeout_ms > 0 {
                config.init_timeout_ms
            } else {
                super::DEFAULT_INIT_TIMEOUT_MS
            }),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            awaiting_init_message: AtomicBool::new(false),
            http_specs_adapter: Arc::new(fallback_adapter),
            cancel_poll_notify: Arc::new(Notify::new()),
        }
    }

    fn spawn_poll_from_statsig_thread(
        http_spec_adapter: Arc<StatsigHttpSpecsAdapter>,
        cancel_notify: Arc<Notify>,
        shutdown_notify: Arc<Notify>,
        shutdown_generation: Arc<AtomicU64>,
        current_shutdown_generation: u64,
    ) {
        let weak_http_adapter = Arc::downgrade(&http_spec_adapter);
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(3000)) => {
                        if let Some(strong_http_adapter) = weak_http_adapter.upgrade() {
                            StatsigHttpSpecsAdapter::run_background_sync(strong_http_adapter).await;
                        } else {
                            log_e!(TAG, "GRPC adapter lost strong reference to StatsigHttpSpecsAdapter. Stopping polling thread");
                            break;
                        }
                    }
                    _ = cancel_notify.notified() => {
                        log_d!(TAG, "Cancel grpc fallback background specs sync");
                        break;
                    }
                    _ = shutdown_notify.notified() => {
                        if shutdown_generation.load(Ordering::SeqCst)
                            != current_shutdown_generation
                        {
                            log_d!(TAG, "Shutting down grpc fallback specs background sync");
                            break;
                        }
                    }
                }
            }
        });
    }

    async fn spawn_grpc_streaming_thread(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        ops_stats: Arc<OpsStatsForInstance>,
    ) -> Result<tokio::task::Id, StatsigErr> {
        let weak_self = Arc::downgrade(&self);
        let shutdown_generation = self.shutdown_generation.load(Ordering::SeqCst);

        statsig_runtime.spawn(BG_TASK_TAG, move |_shutdown_notify| async move {
            if let Some(strong_self) = weak_self.upgrade() {
                if let Err(e) = strong_self
                    .run_retryable_grpc_stream(shutdown_generation)
                    .await
                {
                    log_error_to_statsig_and_console!(
                        &ops_stats,
                        TAG,
                        StatsigErr::GrpcError(format!("gRPC streaming thread failed: {e}"))
                    );
                }
            } else {
                log_error_to_statsig_and_console!(
                    &ops_stats,
                    TAG,
                    StatsigErr::GrpcError(
                        "Failed to upgrade weak reference to strong reference".to_string()
                    )
                );
            }
        })
    }

    async fn run_retryable_grpc_stream(&self, shutdown_generation: u64) -> Result<(), StatsigErr> {
        loop {
            tokio::select! {
                result = self.handle_grpc_request_stream() => {
                    if let Err(err) = result {
                        let attempt = self.retry_state.retry_attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt > RETRY_LIMIT {
                            log_error_to_statsig_and_console!(&self.ops_stats, TAG, StatsigErr::GrpcError(format!("gRPC stream failure, exhaust retry limit: {err:?}")));
                           break;
                        }
                        if attempt == FALL_BACK_TO_POLLING_THREASHOLD {
                            log_d!(TAG, "SFP is not reachable after {} tries: Falling back to polling from statsig", FALL_BACK_TO_POLLING_THREASHOLD);
                            Self::spawn_poll_from_statsig_thread(
                                self.http_specs_adapter.clone(),
                                self.cancel_poll_notify.clone(),
                                self.shutdown_notify.clone(),
                                self.shutdown_generation.clone(),
                                shutdown_generation,
                            );
                        }
                        self.grpc_client.reset_client();

                        // Update retry state
                        let curr_backoff = self.retry_state.backoff_interval_ms.load(Ordering::SeqCst);
                        let new_backoff = if curr_backoff < MAX_BACKOFF_INTERVAL_MS {
                            cmp::min(curr_backoff * DEFAULT_BACKOFF_MULTIPLIER, MAX_BACKOFF_INTERVAL_MS)
                        } else  {
                            MAX_BACKOFF_INTERVAL_MS
                        };
                        self.retry_state.backoff_interval_ms.store(new_backoff,Ordering::SeqCst);
                        self.retry_state.is_retrying.store(true, Ordering::SeqCst);
                        self.log_streaming_err(err, attempt, curr_backoff);
                        tokio::time::sleep(Duration::from_millis(curr_backoff)).await;
                    }
                },
                _ = self.shutdown_notify.notified() => {
                    if self.shutdown_generation.load(Ordering::SeqCst) != shutdown_generation {
                        log_d!(TAG, "Received shutdown signal, stopping stream listener.");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn signal_shutdown(&self) {
        self.shutdown_generation.fetch_add(1, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        self.shutdown_notify.notify_one();
    }

    async fn handle_grpc_request_stream(&self) -> Result<(), StatsigErr> {
        self.grpc_client
            .connect_client()
            .await
            .map_err(|e| StatsigErr::GrpcError(format!("{e}")))?;
        let specs_info = self.get_current_specs_info();
        let mut stream = self
            .grpc_client
            .get_specs_stream(specs_info.as_ref().and_then(|s| s.lcut))
            .await
            .map_err(|e| StatsigErr::GrpcError(format!("{e}")))?;
        loop {
            match stream.message().await {
                Ok(Some(config_spec)) => {
                    self.cancel_poll_notify.notify_one();
                    if self.retry_state.is_retrying.load(Ordering::SeqCst) {
                        // Reset retry state
                        self.retry_state.is_retrying.store(false, Ordering::SeqCst);
                        self.retry_state.retry_attempts.store(0, Ordering::SeqCst);
                        self.retry_state
                            .backoff_interval_ms
                            .store(DEFAULT_BACKOFF_INTERVAL_MS, Ordering::SeqCst);
                        self.ops_stats.log(ObservabilityEvent::new_event(
                            MetricType::Increment,
                            "grpc_reconnected".to_string(),
                            1.0,
                            None,
                        ));
                    }
                    let _ = self
                        .initialization_tx
                        .send(self.handle_streamed_spec(config_spec.spec));
                    self.ops_stats.log(ObservabilityEvent::new_event(
                        MetricType::Increment,
                        "grpc_received_message".to_string(),
                        1.0,
                        None,
                    ));
                }
                err => {
                    return Err(StatsigErr::GrpcError(format!(
                        "Error while receiving stream: {err:?}"
                    )));
                }
            }
        }
    }

    fn set_task_handle(&self, handle_id: tokio::task::Id) {
        self.task_handle.store(Some(Arc::new(ProcessLocalGrpcTask {
            pid: std::process::id(),
            handle_id,
        })));
    }

    fn handle_streamed_spec(&self, data: String) -> Result<(), StatsigErr> {
        let context = if self.awaiting_init_message.swap(false, Ordering::SeqCst) {
            self.add_diagnostics_marker(
                ContextType::Initialize,
                ActionType::End,
                StepType::NetworkRequest,
                Some(true),
            );
            ContextType::Initialize
        } else {
            // Later messages are pushes, not requests, so they only get process markers.
            ContextType::ConfigSync
        };

        self.add_diagnostics_marker(context, ActionType::Start, StepType::Process, None);
        let result = self.send_spec_update_to_listener(data);
        self.add_diagnostics_marker(
            context,
            ActionType::End,
            StepType::Process,
            Some(result.is_ok()),
        );

        if context == ContextType::ConfigSync {
            self.ops_stats
                .enqueue_diagnostics_event(Some(KeyType::DownloadConfigSpecs), context);
        }
        result
    }

    fn end_init_network_marker_on_failure(&self) {
        if self.awaiting_init_message.swap(false, Ordering::SeqCst) {
            self.add_diagnostics_marker(
                ContextType::Initialize,
                ActionType::End,
                StepType::NetworkRequest,
                Some(false),
            );
        }
    }

    fn add_diagnostics_marker(
        &self,
        context: ContextType,
        action: ActionType,
        step: StepType,
        success: Option<bool>,
    ) {
        let mut marker = Marker::new(KeyType::DownloadConfigSpecs, action, Some(step))
            .with_source(grpc_specs_source().to_string());
        if let Some(success) = success {
            marker = marker.with_is_success(success);
        }
        self.ops_stats.add_marker(marker, context);
    }

    fn send_spec_update_to_listener(&self, data: String) -> Result<(), StatsigErr> {
        let listener = self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
            .ok_or_else(|| {
                StatsigErr::LockFailure("Failed to acquire read lock on listener".to_string())
            })?;

        if let Some(listener) = listener.as_ref() {
            let update = SpecsUpdate {
                data: ResponseData::from_bytes(data.into_bytes()),
                source: grpc_specs_source(),
                received_at: Utc::now().timestamp_millis() as u64,
                source_api: None,
            };

            listener.did_receive_specs_update(update)
        } else {
            Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()))
        }
    }

    fn get_current_specs_info(&self) -> Option<SpecsInfo> {
        match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => Some(listener.get_current_specs_info()),
                None => {
                    log_w!(TAG, "Failed to get current lcut");
                    None
                }
            },
            None => {
                log_w!(TAG, "Failed to get current lcut");
                None
            }
        }
    }

    fn log_streaming_err(&self, err: StatsigErr, retry_attempts: u64, backoff: u64) {
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            "grpc_streaming_failed_with_retry_ct".to_string(),
            retry_attempts as f64,
            None,
        ));
        log_w!(
            TAG,
            "gRPC stream failure ({}). Will wait {} ms and retry. Error: {:?}",
            retry_attempts,
            backoff,
            err
        );
    }
}

fn grpc_specs_source() -> SpecsSource {
    SpecsSource::Adapter("GRPC".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk_diagnostics::test_utils::{summarize_marker, RecordingDiagnosticsObserver};
    use crate::SpecsAdapterType;

    struct AcceptSpecs;

    impl SpecsUpdateListener for AcceptSpecs {
        fn did_receive_specs_update(&self, _update: SpecsUpdate) -> Result<(), StatsigErr> {
            Ok(())
        }

        fn get_current_specs_info(&self) -> SpecsInfo {
            SpecsInfo::empty()
        }
    }

    fn create_adapter(sdk_key: &str) -> StatsigGrpcSpecsAdapter {
        let config = SpecAdapterConfig {
            adapter_type: SpecsAdapterType::NetworkGrpcWebsocket,
            specs_url: Some("http://localhost:1".to_string()),
            init_timeout_ms: 3000,
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            domain_name: None,
        };
        let adapter = StatsigGrpcSpecsAdapter::new(sdk_key, &config, None);
        adapter.initialize(Arc::new(AcceptSpecs));
        adapter
    }

    fn summarize(markers: &[(String, serde_json::Value)]) -> Vec<String> {
        markers
            .iter()
            .map(|(context, m)| format!("{context} {}", summarize_marker(m)))
            .collect()
    }

    #[tokio::test]
    async fn test_first_streamed_spec_is_initialize_and_later_ones_are_config_sync() {
        let sdk_key = "secret-grpc-init-diagnostics";
        let recorder = RecordingDiagnosticsObserver::subscribe(sdk_key);
        let adapter = create_adapter(sdk_key);
        adapter.awaiting_init_message.store(true, Ordering::SeqCst);

        adapter.handle_streamed_spec("{}".to_string()).unwrap();
        adapter.handle_streamed_spec("{}".to_string()).unwrap();
        // initialize already resolved, so a late failure must not add another end marker
        adapter.end_init_network_marker_on_failure();

        let markers = recorder.wait_for_markers(5).await;
        assert_eq!(
            summarize(&markers),
            vec![
                "initialize download_config_specs:network_request:end",
                "initialize download_config_specs:process:start",
                "initialize download_config_specs:process:end",
                // pushes are not requests, so only process markers
                "config_sync download_config_specs:process:start",
                "config_sync download_config_specs:process:end",
            ]
        );
        for (_, marker) in &markers {
            assert_eq!(marker["source"], "Adapter(GRPC)");
        }
        assert_eq!(markers[0].1["success"], true);
        assert_eq!(markers[2].1["success"], true);
        assert_eq!(markers[4].1["success"], true);
    }

    #[tokio::test]
    async fn test_initialize_failure_ends_network_marker_once() {
        let sdk_key = "secret-grpc-init-failure-diagnostics";
        let recorder = RecordingDiagnosticsObserver::subscribe(sdk_key);
        let adapter = create_adapter(sdk_key);
        adapter.awaiting_init_message.store(true, Ordering::SeqCst);

        adapter.end_init_network_marker_on_failure();
        adapter.end_init_network_marker_on_failure();
        // a message arriving after initialize gave up is a config_sync push
        adapter.handle_streamed_spec("{}".to_string()).unwrap();

        recorder.wait_for_markers(3).await;
        // give any stray markers a chance to arrive before asserting there are none
        tokio::time::sleep(Duration::from_millis(100)).await;
        let markers = recorder.markers();
        assert_eq!(
            summarize(&markers),
            vec![
                "initialize download_config_specs:network_request:end",
                "config_sync download_config_specs:process:start",
                "config_sync download_config_specs:process:end",
            ]
        );
        assert_eq!(markers[0].1["success"], false);
    }
}
