use async_trait::async_trait;
use chrono::Utc;
use sigstat::{
    log_d, log_e, log_w, SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr,
};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tonic::codegen::tokio_stream::StreamExt;
use crate::statsig_grpc_client::StatsigGrpcClient;

const DEFAULT_BACKOFF_INTERVAL_MS: u64 = 3000;
const RETRY_LIMIT: u64 = 10;

pub mod statsig_forward_proxy {
    tonic::include_proto!("statsig_forward_proxy");
}

pub struct StatsigGrpcSpecAdapter {
    backoff_interval_ms: u64,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
    grpc_client: StatsigGrpcClient,
}

#[async_trait]
impl SpecsAdapter for StatsigGrpcSpecAdapter {
    async fn start(
        self: Arc<Self>,
        runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        self.grpc_client.connect_client().await?;

        self.set_listener(listener)?;

        let handle = self
            .clone()
            .spawn_grpc_streaming_thread(runtime_handle)
            .await?;

        self.set_task_handle(handle)
    }

    async fn manually_sync_specs(&self, current_store_lcut: Option<u64>) -> Result<(), StatsigErr> {
        let response = self.grpc_client.get_specs(current_store_lcut).await?;
        self.send_spec_update_to_listener(response.spec)?;
        Ok(())
    }

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let task_handle = self.task_handle.lock().map_err(|_| {
            StatsigErr::CustomError("Failed to acquire lock to running task".to_string())
        })?.take();

        if let Some(handle) = task_handle {
            if tokio::time::timeout(timeout, handle).await.is_err() {
                return Err(StatsigErr::CustomError(
                    "Failed to gracefully shutdown StatsigGrpcSpecsAdapter.".to_string(),
                ));
            }
        } else {
            return Err(StatsigErr::CustomError("No running task to shut down".to_string()));
        }

        Ok(())
    }
}

impl StatsigGrpcSpecAdapter {
    pub fn new(
        sdk_key: &str,
        proxy_api: &str,
        backoff_interval_ms: Option<u64>
    ) -> Self {
        Self {
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle: Mutex::new(None),
            grpc_client: StatsigGrpcClient::new(sdk_key, proxy_api),
            backoff_interval_ms: backoff_interval_ms.unwrap_or(DEFAULT_BACKOFF_INTERVAL_MS)
        }
    }

    async fn spawn_grpc_streaming_thread(
        self: Arc<Self>,
        runtime_handle: &Handle,
    ) -> Result<JoinHandle<()>, StatsigErr> {
        let weak_self = Arc::downgrade(&self);

        Ok(runtime_handle.spawn(async move {
            if let Some(strong_self) = weak_self.upgrade() {
                if let Err(e) = strong_self.run_retryable_grpc_stream().await {
                    log_e!("gRPC streaming thread failed: {}", e);
                }
            } else {
                log_e!("Failed to upgrade weak reference to strong reference");
            }
        }))
    }

    async fn run_retryable_grpc_stream(&self) -> Result<(), StatsigErr> {
        let mut retries = 0;

        loop {
            tokio::select! {
                result = self.handle_grpc_request_stream() => {
                    if let Err(err) = result {
                        if retries > RETRY_LIMIT {
                           log_e!("gRPC stream failure: {:?}", err);
                           break;
                        }
                        retries += 1;

                        self.grpc_client.reset_client();

                        let backoff = self.backoff_interval_ms * retries * retries;

                        log_w!("gRPC stream failure ({}). Will wait {} ms and retry. Error: {:?}", retries, backoff, err);
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                    } else {
                        retries = 0;
                    }
                },
                _ = self.shutdown_notify.notified() => {
                    log_d!("Received shutdown signal, stopping stream listener.");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_grpc_request_stream(&self) -> Result<(), StatsigErr> {
        let lcut = self.get_current_store_lcut();
        let mut stream = self.grpc_client.get_specs_stream(lcut).await?;

        while let Some(config_spec_result) = stream.next().await {
            match config_spec_result {
                Ok(config_spec) => {
                    let _ = self.send_spec_update_to_listener(config_spec.spec);
                }
                Err(e) => {
                    log_e!("Error while receiving stream: {:?}", e);
                    return Err(StatsigErr::SpecsAdapterLockFailure);
                }
            }
        }

        Ok(())
    }

    fn set_task_handle(&self, handle: JoinHandle<()>) -> Result<(), StatsigErr> {
        let mut guard = self.task_handle
            .lock().map_err(|_| StatsigErr::BackgroundTaskLockFailure)?;

        *guard = Some(handle);
        Ok(())
    }

    fn set_listener(
        &self,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        let mut mut_listener = self.listener.write().map_err(|_| StatsigErr::SpecsListenerLockFailure)?;

        *mut_listener = Some(listener);
        Ok(())
    }

    fn send_spec_update_to_listener(&self, data: String) -> Result<(), StatsigErr> {
        let listener = self.listener
            .read().map_err(|_| StatsigErr::BackgroundTaskLockFailure)?;

        if let Some(listener) = listener.as_ref() {
            let update = SpecsUpdate {
                data,
                source: SpecsSource::Network,
                received_at: Utc::now().timestamp_millis() as u64,
            };
            listener.did_receive_specs_update(update);
            Ok(())
        } else {
            Err(StatsigErr::SpecsListenerNotSet)
        }
    }

    fn get_current_store_lcut(&self) -> Option<u64> {
        if let Some(listener) = self.listener.read().ok() {
            if let Some(listener) = listener.as_ref() {
                return listener.get_current_specs_info().lcut;
            }
        }

        log_w!("Failed to get current lcut");
        None
    }
}

