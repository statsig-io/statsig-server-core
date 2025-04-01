use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::{
    log_d, log_error_to_statsig_and_console, log_w, SpecAdapterConfig, SpecsAdapter, SpecsSource,
    SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use async_trait::async_trait;
use chrono::Utc;
use sigstat_grpc::statsig_grpc_client::StatsigGrpcClient;
use std::cmp;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, Notify};
use tokio::time::{sleep, timeout};

use super::StatsigHttpSpecsAdapter;
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

const TAG: &str = stringify!(StatsigGrpcSpecsAdapter);
const BG_TASK_TAG: &str = "grpc_streaming";

pub struct StatsigGrpcSpecsAdapter {
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
    initialization_tx: Arc<broadcast::Sender<Result<(), StatsigErr>>>,
    task_handle_id: Mutex<Option<tokio::task::Id>>,
    grpc_client: StatsigGrpcClient,
    retry_state: StreamingRetryState,
    init_timeout: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
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
        let handle_id = self
            .clone()
            .spawn_grpc_streaming_thread(statsig_runtime, self.ops_stats.clone())
            .await?;

        self.set_task_handle_id(handle_id)?;
        let mut rx = self.initialization_tx.subscribe();
        match timeout(self.init_timeout, rx.recv()).await {
            Ok(res) => match res {
                Ok(Ok(())) => Ok(()),
                Ok(Err(err)) => Err(StatsigErr::GrpcError(format!(
                    "Failed to initialize from streaming: {}",
                    err
                ))),
                Err(_) => Err(StatsigErr::GrpcError("Failed to get a ".to_string())),
            },
            Err(_) => Err(StatsigErr::GrpcError(
                "Start Timeout to get a response".to_string(),
            )),
        }
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let ops_stats = self.ops_stats.clone();
        let self_clone = self.clone();
        let self_clone_clone = self.clone();

        let lock = self_clone.task_handle_id.lock().await;
        if lock.is_some() {
            return Ok(());
        }
        drop(lock); // Release the lock before spawning

        let task_id = self_clone_clone
            .spawn_grpc_streaming_thread(statsig_runtime, ops_stats)
            .await?;

        let mut new_lock = self_clone.task_handle_id.lock().await;
        *new_lock = Some(task_id);
        Ok(())
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        self.http_specs_adapter.initialize(listener.clone());
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats,
                    TAG,
                    "Failed to acquire write lock on listener: {}",
                    e
                );
            }
        }
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let opt_handle_id = self
            .task_handle_id
            .try_lock()
            .map_err(|_| {
                StatsigErr::GrpcError("Failed to acquire lock to running task".to_string())
            })?
            .take();

        let handle_id = match opt_handle_id {
            Some(handle_id) => handle_id,
            None => {
                return Err(StatsigErr::ThreadFailure(
                    "No running task found".to_string(),
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
    pub fn new(sdk_key: &str, config: &SpecAdapterConfig, disable_network: Option<bool>) -> Self {
        let fallback_adapter =
            StatsigHttpSpecsAdapter::new(sdk_key, None, false, None, disable_network);
        let (init_tx, _) = broadcast::channel(1);
        Self {
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle_id: Mutex::new(None),
            grpc_client: StatsigGrpcClient::new(
                sdk_key,
                &config.specs_url.clone().unwrap_or("INVALID".to_owned()),
            ),
            initialization_tx: Arc::new(init_tx),
            retry_state: StreamingRetryState {
                backoff_interval_ms: DEFAULT_BACKOFF_INTERVAL_MS.into(),
                retry_attempts: 0.into(),
                is_retrying: false.into(),
            },
            init_timeout: Duration::from_millis(config.init_timeout_ms),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            http_specs_adapter: Arc::new(fallback_adapter),
            cancel_poll_notify: Arc::new(Notify::new()),
        }
    }

    fn spawn_poll_from_statsig_thread(
        http_spec_adapter: Arc<StatsigHttpSpecsAdapter>,
        cancel_notify: Arc<Notify>,
        shutdown_notify: Arc<Notify>,
    ) {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(3000)) => {
                        StatsigHttpSpecsAdapter::run_background_sync(&Arc::downgrade(&http_spec_adapter)).await;
                    }
                    _ = cancel_notify.notified() => {
                        log_d!(TAG, "Cancel grpc fallback background specs sync");
                        break;
                    }
                    _ = shutdown_notify.notified() => {
                        log_d!(TAG, "Shutting down grpc fallback specs background sync");
                        break;
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

        Ok(
            statsig_runtime.spawn(BG_TASK_TAG, |_shutdown_notify| async move {
                if let Some(strong_self) = weak_self.upgrade() {
                    if let Err(e) = strong_self.run_retryable_grpc_stream().await {
                        log_error_to_statsig_and_console!(
                            &ops_stats,
                            TAG,
                            "gRPC streaming thread failed: {}",
                            e
                        );
                    }
                } else {
                    log_error_to_statsig_and_console!(
                        &ops_stats,
                        TAG,
                        "Failed to upgrade weak reference to strong reference"
                    );
                }
            }),
        )
    }

    async fn run_retryable_grpc_stream(&self) -> Result<(), StatsigErr> {
        loop {
            tokio::select! {
                result = self.handle_grpc_request_stream() => {
                    if let Err(err) = result {
                        let attempt = self.retry_state.retry_attempts.fetch_add(1, Ordering::SeqCst);
                        if attempt > RETRY_LIMIT {
                            log_error_to_statsig_and_console!(&self.ops_stats, TAG, "gRPC stream failure, exhaust retry limit: {:?}", err);
                           break;
                        }
                        if attempt == FALL_BACK_TO_POLLING_THREASHOLD {
                            log_d!(TAG, "SFP is not reachable after {} tries: Falling back to polling from statsig", FALL_BACK_TO_POLLING_THREASHOLD);
                            Self::spawn_poll_from_statsig_thread(self.http_specs_adapter.clone(), self.cancel_poll_notify.clone(), self.shutdown_notify.clone());
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
                    log_d!(TAG, "Received shutdown signal, stopping stream listener.");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_grpc_request_stream(&self) -> Result<(), StatsigErr> {
        self.grpc_client
            .connect_client()
            .await
            .map_err(|e| StatsigErr::GrpcError(format!("{}", e)))?;
        let lcut = self.get_current_store_lcut();
        let mut stream = self
            .grpc_client
            .get_specs_stream(lcut)
            .await
            .map_err(|e| StatsigErr::GrpcError(format!("{}", e)))?;
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
                        .send(self.send_spec_update_to_listener(config_spec.spec));
                    self.ops_stats.log(ObservabilityEvent::new_event(
                        MetricType::Increment,
                        "grpc_received_message".to_string(),
                        1.0,
                        None,
                    ));
                }
                err => {
                    return Err(StatsigErr::GrpcError(format!(
                        "Error while receiving stream: {:?}",
                        err
                    )));
                }
            }
        }
    }

    fn set_task_handle_id(&self, handle_id: tokio::task::Id) -> Result<(), StatsigErr> {
        let mut guard = self
            .task_handle_id
            .try_lock()
            .map_err(|e| StatsigErr::LockFailure(e.to_string()))?;

        *guard = Some(handle_id);
        Ok(())
    }

    fn send_spec_update_to_listener(&self, data: String) -> Result<(), StatsigErr> {
        let listener = self
            .listener
            .read()
            .map_err(|e| StatsigErr::LockFailure(e.to_string()))?;

        if let Some(listener) = listener.as_ref() {
            let update = SpecsUpdate {
                data,
                source: SpecsSource::Adapter("GRPC".to_string()),
                received_at: Utc::now().timestamp_millis() as u64,
            };

            listener.did_receive_specs_update(update)
        } else {
            Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()))
        }
    }

    fn get_current_store_lcut(&self) -> Option<u64> {
        if let Ok(listener) = self.listener.read() {
            if let Some(listener) = listener.as_ref() {
                return listener.get_current_specs_info().lcut;
            }
        }

        log_w!(TAG, "Failed to get current lcut");
        None
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
