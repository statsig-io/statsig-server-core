use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use crate::{StatsigErr, StatsigHttpSpecsAdapter};
use tokio::runtime::Handle;
use crate::{SpecsAdapter, SpecsUpdateListener, log_w};
use super::{SpecAdapterConfig, AdapterType};

#[cfg(feature = "with_grpc")]
use super::statsig_grpc_specs_adapter::StatsigGrpcSpecAdapter;

#[cfg(not(feature = "with_grpc"))]
use crate::{log_e};

pub struct StatsigCustomizedSpecsAdapter {
    adapters: Vec<Arc<dyn SpecsAdapter>>,
}

impl StatsigCustomizedSpecsAdapter {
    pub fn new(sdk_key: &str, configs: Vec<SpecAdapterConfig>) -> Self {
        let mut adapters: Vec<Arc<dyn SpecsAdapter>> = Vec::new();
        for (_, config) in configs.iter().enumerate() {
            match config.adapter_type {
                AdapterType::NetworkGrpcWebsocket => {
                    if let Some(adapter) = Self::create_grpc_adapter(sdk_key, config){
                        adapters.push(adapter);
                    }
                }
                AdapterType::NetworkHttp => {
                    // Since strategies is an order list, we will just use i
                    adapters.push(Arc::new(StatsigHttpSpecsAdapter::new(
                        sdk_key,
                        Some(&config.specs_url),
                        config.init_timeout_ms,
                        None,
                    )))
                }
            }
        }

        StatsigCustomizedSpecsAdapter { adapters }
    }
  
  #[cfg(feature = "with_grpc")]
  fn create_grpc_adapter(sdk_key: &str,config: &SpecAdapterConfig) -> Option<Arc<dyn SpecsAdapter>> {
    Some(Arc::new(StatsigGrpcSpecAdapter::new(
                          sdk_key,
                          config                        
                      )))
  }
  
  #[cfg(not(feature = "with_grpc"))]
  fn create_grpc_adapter(_sdk_key: &str, _config: &SpecAdapterConfig) -> Option<Arc<dyn SpecsAdapter>> {
    log_e!("Trying to use grpc websocket adapter but with grpc feature is not enabled");
    None
  }
}

#[async_trait]
impl SpecsAdapter for StatsigCustomizedSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        for adapter in &self.adapters {
            match adapter
                .to_owned()
                .start(runtime_handle, listener.clone())
                .await
            {
                Ok(()) => {
                    return Ok(());
                }
                _ => {
                    log_w!("Failed to initialize from {} adapter", adapter.get_type_name());
                    // Carry on
                }
            }
        }
        return Err(StatsigErr::SpecsAdapterNetworkFailure);
    }

    fn schedule_background_sync(self: Arc<Self>, runtime_handle: &Handle) -> Result<(), StatsigErr> {
        // TODO: we probably should have another option for config sync sources, but for now, we only have one
        self.adapters[0]
            .clone()
            .schedule_background_sync(runtime_handle)
    }

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        let timeout_for_each = timeout
            .checked_div(self.adapters.len() as u32)
            .unwrap_or(timeout);
        for adapter in &self.adapters {
            let _ = adapter.shutdown(timeout_for_each).await;
        }
        Ok(())
    }

    fn get_type_name(&self) -> String {
        "StatsigCustomizedSpecsAdapter".to_string()
    }
}
