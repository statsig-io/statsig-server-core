use tonic::transport::Channel;
use std::sync::Mutex;
use std::time::Duration;
use tonic::Streaming;
use sigstat::StatsigErr;
use crate::{ConfigSpecRequest, ConfigSpecResponse};
use crate::statsig_grpc_specs_adapter::statsig_forward_proxy::config_spec_request::ApiVersion;
use crate::statsig_grpc_specs_adapter::statsig_forward_proxy::statsig_forward_proxy_client::StatsigForwardProxyClient;

pub struct StatsigGrpcClient {
    sdk_key: String,
    proxy_api: String,
    grpc_client: Mutex<Option<StatsigForwardProxyClient<Channel>>>,
}

impl StatsigGrpcClient {
    pub fn new(sdk_key: &str, proxy_api: &str) -> Self {
        Self {
            sdk_key: sdk_key.to_string(),
            proxy_api: proxy_api.to_string(),
            grpc_client: Mutex::new(None),
        }
    }

    pub async fn connect_client(&self) -> Result<(), StatsigErr> {
        self.get_or_setup_grpc_client().await.map(|_| ())
    }

    pub fn reset_client(&self) {
        if let Ok(mut lock) = self.grpc_client.lock() {
            *lock = None;
        }
    }

    pub async fn get_specs(&self, lcut: Option<u64>) -> Result<ConfigSpecResponse, StatsigErr> {
        let request = create_config_spec_request(&self.sdk_key, lcut);
        let mut client = self.get_or_setup_grpc_client().await?;

        client.get_config_spec(request).await
            .map_err(|e| StatsigErr::CustomError(format!("gRPC get_config_spec failed - {}", e)))
            .map(|r| r.into_inner())
    }
    pub async fn get_specs_stream(&self, lcut: Option<u64>) -> Result<Streaming<ConfigSpecResponse>, StatsigErr> {
        let request = create_config_spec_request(&self.sdk_key, lcut);
        let mut client = self.get_or_setup_grpc_client().await?;

        client.stream_config_spec(request).await
            .map_err(|e| StatsigErr::CustomError(format!("gRPC stream_config_spec failed - {}", e)))
            .map(|s| s.into_inner())
    }

    async fn get_or_setup_grpc_client(&self) -> Result<StatsigForwardProxyClient<Channel>, StatsigErr> {
        {
            let lock = self.grpc_client
                .lock().map_err(|_| StatsigErr::SpecsAdapterLockFailure)?;

            if let Some(client) = lock.as_ref() {
                return Ok(client.clone());
            }
        }

        let channel = Channel::from_shared(self.proxy_api.clone())
            .map_err(|e| StatsigErr::CustomError(format!("Failed to create gRPC channel: {}", e)))?
            .connect_timeout(Duration::from_secs(5))
            .connect()
            .await
            .map_err(|e| StatsigErr::CustomError(format!("gRPC failed to connect: {}", e)))?;

        let new_client = StatsigForwardProxyClient::new(channel);

        let mut lock = self.grpc_client
            .lock().map_err(|_| StatsigErr::SpecsAdapterLockFailure)?;

        *lock = Some(new_client.clone());
        Ok(new_client)
    }
}

fn create_config_spec_request(sdk_key: &String, current_lcut: Option<u64>) -> ConfigSpecRequest {
    ConfigSpecRequest {
        since_time: current_lcut,
        sdk_key: sdk_key.clone(),
        version: Some(ApiVersion::V2 as i32),
    }
}