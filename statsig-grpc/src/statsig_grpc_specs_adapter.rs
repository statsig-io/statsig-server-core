use async_trait::async_trait;
use statsig::{log_d, log_e, log_w, SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr};
use statsig_forward_proxy::statsig_forward_proxy_client::StatsigForwardProxyClient;
use statsig_forward_proxy::ConfigSpecRequest;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::{Notify};
use tokio::task::JoinHandle;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::transport::Channel;
use crate::statsig_grpc_specs_adapter::statsig_forward_proxy::ConfigSpecResponse;
use crate::statsig_grpc_specs_adapter::statsig_forward_proxy::config_spec_request::ApiVersion;
use chrono::Utc;

pub mod statsig_forward_proxy {
    tonic::include_proto!("statsig_forward_proxy");
}

pub struct StatsigGrpcSpecAdapter {
    sdk_key: String,
    proxy_api: String,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
    grpc_client: Mutex<Option<StatsigForwardProxyClient<Channel>>>
}

#[async_trait]
impl SpecsAdapter for StatsigGrpcSpecAdapter {
    async fn start(
        self: Arc<Self>,
        runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        self.setup_grpc_client().await?;
        self.set_listener(listener)?;
        let handle = self.clone().spawn_grpc_streaming_thread(runtime_handle).await?;
        self.set_task_handle(handle)
    }

    async fn manually_sync_specs(&self, current_store_lcut: Option<u64>) -> Result<(), StatsigErr> {
        let request = create_config_spec_request(&self.sdk_key, current_store_lcut);

        let mut client = self.get_grpc_client()?;
        let response = client.get_config_spec(request)
            .await.map_err(|e| StatsigErr::CustomError(format!("gRPC get_config_spec failed - {}", e)))?
            .into_inner();

        self.send_spec_update_to_listener(response.spec)?;

        Ok(())
    }

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let task_handle = {
            match self.task_handle.lock() {
                Ok(mut guard) => guard.take(),
                Err(_) => return Err(StatsigErr::CustomError("Failed to acquire lock to running task".to_string())),
            }
        };

        if let Some(handle) = task_handle {
            if tokio::time::timeout(timeout, handle).await.is_err() {
                return Err(StatsigErr::CustomError("Failed to gracefully shutdown StatsigGrpcSpecsAdapter.".to_string()));
            }
        } else {
            return Err(StatsigErr::CustomError("No running task to shut down".to_string()));
        }

        Ok(())
    }
}

impl StatsigGrpcSpecAdapter {
    pub fn new(sdk_key: &str, proxy_api: &str) -> Self {
        Self {
            sdk_key: sdk_key.to_string(),
            proxy_api: proxy_api.to_string(),
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle: Mutex::new(None),
            grpc_client: Mutex::new(None)
        }
    }

    async fn setup_grpc_client(&self) -> Result<(), StatsigErr> {
        let channel = Channel::from_shared(self.proxy_api.clone())
            .map_err(|e| StatsigErr::CustomError(format!("Failed to create gRPC channel: {}", e)))?
            .connect_timeout(Duration::from_secs(5))
            .connect().await
            .map_err(|e| StatsigErr::CustomError(format!("gRPC failed to connect: {}", e)))?;

        match self.grpc_client.lock() {
            Ok(mut lock) => Ok(*lock = Some(StatsigForwardProxyClient::new(channel))),
            Err(_) => Err(StatsigErr::SpecsAdapterLockFailure)
        }
    }

    fn get_grpc_client(&self) -> Result<StatsigForwardProxyClient<Channel>, StatsigErr> {
        match self.grpc_client.lock() {
            Ok(lock) => match &*lock {
                Some(client) => Ok(client.clone()),
                None => Err(StatsigErr::CustomError("No gRPC client found".to_string()))
            },
            Err(_) => Err(StatsigErr::CustomError("Failed to acquire gRPC client lock".to_string()))
        }
    }

    async fn spawn_grpc_streaming_thread(
        self: Arc<Self>,
        runtime_handle: &Handle,
    ) -> Result<JoinHandle<()>, StatsigErr> {
        let shutdown_notify = self.shutdown_notify.clone();
        let weak_self = Arc::downgrade(&self);
        let mut client = self.get_grpc_client()?;

        Ok(runtime_handle.spawn(async move {
            if let Some(strong_self) = weak_self.upgrade() {
                if let Err(err) = strong_self.start_grpc_stream(&mut client).await {
                    log_e!("Failed to start gRPC stream: {:?}", err);
                }
            } else {
                log_e!("Failed to upgrade weak reference to strong reference");
            }

            tokio::select! {
                _ = shutdown_notify.notified() => {
                    log_d!("Received shutdown signal, stopping stream listener.");
                }
            }
        }))
    }

    fn set_listener(&self, listener: Arc<dyn SpecsUpdateListener + Send + Sync>) -> Result<(), StatsigErr> {
        match self.listener.write() {
            Ok(mut mut_listener) => Ok(*mut_listener = Some(listener)),
            Err(_) => Err(StatsigErr::SpecsListenerLockFailure)
        }
    }

    async fn start_grpc_stream(
        self: Arc<Self>,
        client: &mut StatsigForwardProxyClient<Channel>
    ) -> Result<(), StatsigErr> {
        let lcut = self.get_current_store_lcut();
        let request = create_config_spec_request(&self.sdk_key, lcut);

        let mut stream = client.stream_config_spec(request)
            .await.map_err(|e| StatsigErr::CustomError(format!("gRPC stream_config_spec failed - {}", e)))?
            .into_inner();

        while let Some(config_spec_result) = stream.next().await {
            self.handle_stream_result(config_spec_result).await?;
        }

        Ok(())
    }

    async fn handle_stream_result(
        &self,
        config_spec_result: Result<ConfigSpecResponse, tonic::Status>,
    ) -> Result<(), StatsigErr> {
        match config_spec_result {
            Ok(config_spec) => Ok(self.send_spec_update_to_listener(config_spec.spec)?),
            Err(e) => {
                log_e!("Error while receiving stream: {:?}", e);
                return Err(StatsigErr::CustomError("Stream error".to_string()));
            }
        }
    }

    fn set_task_handle(&self, handle: JoinHandle<()>) -> Result<(), StatsigErr> {
        match self.task_handle.lock() {
            Ok(mut guard) => {
                *guard = Some(handle);
                Ok(())
            }
            Err(_) => Err(StatsigErr::BackgroundTaskLockFailure),
        }
    }

    fn send_spec_update_to_listener(&self, data: String) -> Result<(), StatsigErr> {
        let listener = self.listener.read()
            .map_err(|_| StatsigErr::BackgroundTaskLockFailure)?;

        match listener.as_ref() {
            Some(listener) => {
                let update = SpecsUpdate {
                    data,
                    source: SpecsSource::Network,
                    received_at: Utc::now().timestamp_millis() as u64,
                };
                listener.did_receive_specs_update(update);

                Ok(())
            }
            None => Err(StatsigErr::SpecsListenerNotSet)
        }
    }

    fn get_current_store_lcut(&self) -> Option<u64> {
        if let Ok(listener) = self.listener.read() {
            if let Some(listener) = listener.as_ref() {
                return listener.get_current_specs_info().lcut;
            }
        }

        log_w!("Failed to get current lcut");
        return None;
    }
}

fn create_config_spec_request(sdk_key: &String, current_lcut: Option<u64>) -> ConfigSpecRequest {
    ConfigSpecRequest {
        since_time: current_lcut,
        sdk_key: sdk_key.clone(),
        version: Some(ApiVersion::V2 as i32),
    }
}
