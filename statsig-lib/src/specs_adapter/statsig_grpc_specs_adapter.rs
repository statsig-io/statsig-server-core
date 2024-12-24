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
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;
// Todo make those configurable
const DEFAULT_BACKOFF_INTERVAL_MS: u64 = 3000;
const DEFAULT_BACKOFF_MULTIPLIER: u64 = 2;
const MAX_BACKOFF_INTERVAL_MS: u64 = 60 * 1000;
const RETRY_LIMIT: u64 = 10 * 24 * 60 * 60;

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
    initialized_notify: Arc<Notify>,
    task_handle_id: Mutex<Option<tokio::task::Id>>,
    grpc_client: StatsigGrpcClient,
    retry_state: StreamingRetryState,
    init_timeout: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
}

#[async_trait]
impl SpecsAdapter for StatsigGrpcSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        _listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        let handle_id = self
            .clone()
            .spawn_grpc_streaming_thread(statsig_runtime, self.ops_stats.clone())
            .await
            .unwrap();

        self.set_task_handle_id(handle_id)?;

        match timeout(self.init_timeout, self.initialized_notify.notified()).await {
            Ok(_) => Ok(()),
            Err(_) => Err(StatsigErr::GrpcError(
                "Start Timeout to get a response".to_string(),
            )),
        }
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // It should be already started wtihin spawn_grpc_streaming_thread
        Ok(())
    }


    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
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
            .lock()
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

        if tokio::time::timeout(timeout, statsig_runtime.await_join_handle(BG_TASK_TAG, &handle_id))
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
    pub fn new(sdk_key: &str, config: &SpecAdapterConfig) -> Self {
        Self {
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle_id: Mutex::new(None),
            grpc_client: StatsigGrpcClient::new(
                sdk_key,
                &config.specs_url.clone().unwrap_or("INVALID".to_owned()),
            ),
            initialized_notify: Arc::new(Notify::new()),
            retry_state: StreamingRetryState {
                backoff_interval_ms: DEFAULT_BACKOFF_INTERVAL_MS.into(),
                retry_attempts: 0.into(),
                is_retrying: false.into(),
            },
            init_timeout: Duration::from_millis(config.init_timeout_ms),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        }
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
                        log_w!(TAG, "gRPC stream failure ({}). Will wait {} ms and retry. Error: {:?}", attempt, curr_backoff, err);
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
                    self.initialized_notify.notify_one();
                    if self.retry_state.is_retrying.load(Ordering::SeqCst) {
                        // Reset retry state
                        self.retry_state.is_retrying.store(false, Ordering::SeqCst);
                        self.retry_state.retry_attempts.store(0, Ordering::SeqCst);
                        self.retry_state
                            .backoff_interval_ms
                            .store(DEFAULT_BACKOFF_INTERVAL_MS, Ordering::SeqCst);
                    }
                    let _ = self.send_spec_update_to_listener(config_spec.spec);
                }
                _ => {
                    log_w!(TAG, "Error while receiving stream");
                    return Err(StatsigErr::NetworkError(
                        "Error while receiving stream".to_string(),
                    ));
                }
            }
        }
    }

    fn set_task_handle_id(&self, handle_id: tokio::task::Id) -> Result<(), StatsigErr> {
        let mut guard = self
            .task_handle_id
            .lock()
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
}
