use std::{sync::Arc, time::Duration};

use super::statsig_data_store_specs_adapter::StatsigDataStoreSpecsAdapter;
use super::StatsigHttpSpecsAdapter;
use super::{SpecAdapterConfig, SpecsAdapterType};
use crate::data_store_interface::DataStoreTrait;
use crate::hashing::Hashing;
use crate::{log_w, SpecsAdapter, SpecsUpdateListener, StatsigOptions};
use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;

#[cfg(feature = "with_grpc")]
use super::statsig_grpc_specs_adapter::StatsigGrpcSpecsAdapter;

#[cfg(not(feature = "with_grpc"))]
use crate::log_e;

const TAG: &str = stringify!(StatsigCustomizedSpecsAdapter);

pub struct StatsigCustomizedSpecsAdapter {
    adapters: Vec<Arc<dyn SpecsAdapter>>,
}

impl StatsigCustomizedSpecsAdapter {
    pub fn new_from_config(sdk_key: &str, configs: Vec<SpecAdapterConfig>) -> Self {
        let mut adapters: Vec<Arc<dyn SpecsAdapter>> = Vec::new();
        for (_, config) in configs.iter().enumerate() {
            match config.adapter_type {
                SpecsAdapterType::NetworkGrpcWebsocket => {
                    if let Some(adapter) = Self::create_grpc_adapter(sdk_key, config) {
                        adapters.push(adapter);
                    }
                }
                SpecsAdapterType::NetworkHttp => {
                    // Since strategies is an order list, we will just use i
                    adapters.push(Arc::new(StatsigHttpSpecsAdapter::new(
                        sdk_key,
                        Some(&config.specs_url),
                        None,
                    )))
                }
            }
        }

        StatsigCustomizedSpecsAdapter { adapters }
    }

    pub fn new_from_data_store(sdk_key: &str, data_store: Arc<dyn DataStoreTrait>, options: &StatsigOptions, hashing: &Hashing) -> Self {
        let data_adapter_spec_adapter = StatsigDataStoreSpecsAdapter::new(sdk_key, data_store, hashing, options.specs_sync_interval_ms);
        let http_adapter = StatsigHttpSpecsAdapter::new(
            sdk_key,
            options.specs_url.as_ref(),
            options.specs_sync_interval_ms,
        );
        let adapters: Vec<Arc<dyn SpecsAdapter>> = vec![Arc::new(data_adapter_spec_adapter), Arc::new(http_adapter)];
        StatsigCustomizedSpecsAdapter {
            adapters
        }
    }

    #[cfg(feature = "with_grpc")]
    fn create_grpc_adapter(
        sdk_key: &str,
        config: &SpecAdapterConfig,
    ) -> Option<Arc<dyn SpecsAdapter>> {
        Some(Arc::new(StatsigGrpcSpecsAdapter::new(sdk_key, config)))
    }

    #[cfg(not(feature = "with_grpc"))]
    fn create_grpc_adapter(
        _sdk_key: &str,
        _config: &SpecAdapterConfig,
    ) -> Option<Arc<dyn SpecsAdapter>> {
        log_e!(TAG, "Trying to use grpc websocket adapter but with grpc feature is not enabled");
        None
    }
}

#[async_trait]
impl SpecsAdapter for StatsigCustomizedSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        for adapter in &self.adapters {
            match adapter
                .to_owned()
                .start(statsig_runtime, listener.clone())
                .await
            {
                Ok(()) => {
                    return Ok(());
                }
                _ => {
                    log_w!(
                        TAG,
                        "Failed to initialize from {} adapter",
                        adapter.get_type_name()
                    );
                    // Carry on
                }
            }
        }
        return Err(StatsigErr::NetworkError(
            "Failed to start any adapters".to_string(),
        ));
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // TODO: we probably should have another option for config sync sources, but for now, we only have one
        for adapter in &self.adapters {
            match adapter.to_owned().schedule_background_sync(statsig_runtime) {
                Ok(_) => {
                    return Ok(())
                },
                Err(_) => {
                    // Carry on and try  next adapter
                },
            }
        }
        Ok(())
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let timeout_for_each = timeout
            .checked_div(self.adapters.len() as u32)
            .unwrap_or(timeout);
        for adapter in &self.adapters {
            let _ = adapter.shutdown(timeout_for_each, statsig_runtime).await;
        }
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigCustomizedSpecsAdapter).to_string()
    }
}
