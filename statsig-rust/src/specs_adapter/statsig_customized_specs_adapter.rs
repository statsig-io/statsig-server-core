use std::{sync::Arc, time::Duration};

use super::statsig_data_store_specs_adapter::StatsigDataStoreSpecsAdapter;
use super::StatsigHttpSpecsAdapter;
use super::{SpecAdapterConfig, SpecsAdapterType};
use crate::data_store_interface::DataStoreTrait;
use crate::{log_i, log_w, SpecsAdapter, SpecsUpdateListener, StatsigOptions};
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
    pub fn new_from_config(
        sdk_key: &str,
        data_store_key: &str,
        configs: Vec<SpecAdapterConfig>,
        options: &StatsigOptions,
    ) -> Self {
        let mut adapters: Vec<Arc<dyn SpecsAdapter>> = Vec::new();
        for config in &configs {
            match config.adapter_type {
                SpecsAdapterType::NetworkGrpcWebsocket => {
                    if let Some(adapter) = Self::create_grpc_adapter(sdk_key, config, options) {
                        adapters.push(adapter);
                    }
                }
                SpecsAdapterType::NetworkHttp => {
                    adapters.push(Arc::new(Self::http_adapter_from_config(
                        sdk_key, options, config,
                    )));
                }
                SpecsAdapterType::DataStore => match options.data_store.clone() {
                    Some(data_store) => {
                        adapters.push(Arc::new(StatsigDataStoreSpecsAdapter::new(
                            sdk_key,
                            data_store_key,
                            data_store,
                            Some(options),
                        )));
                    }
                    None => log_w!(TAG, "Datastore is not present for syncing spec"),
                },
            }
        }

        StatsigCustomizedSpecsAdapter { adapters }
    }

    pub fn new_from_data_store(
        sdk_key: &str,
        data_store_key: &str,
        data_store: Arc<dyn DataStoreTrait>,
        options: &StatsigOptions,
    ) -> Self {
        let data_store_spec_adapter =
            StatsigDataStoreSpecsAdapter::new(sdk_key, data_store_key, data_store, Some(options));
        let http_adapter = StatsigHttpSpecsAdapter::new(sdk_key, Some(options), None);
        let adapters: Vec<Arc<dyn SpecsAdapter>> =
            vec![Arc::new(data_store_spec_adapter), Arc::new(http_adapter)];
        StatsigCustomizedSpecsAdapter { adapters }
    }

    fn http_adapter_from_config(
        sdk_key: &str,
        options: &StatsigOptions,
        config: &SpecAdapterConfig,
    ) -> StatsigHttpSpecsAdapter {
        let mut adapter =
            StatsigHttpSpecsAdapter::new(sdk_key, Some(options), config.specs_url.clone());
        // The request layer treats 0 as "no init timeout", so 0 must not reach it.
        adapter.set_init_timeout_ms(if config.init_timeout_ms > 0 {
            config.init_timeout_ms
        } else {
            super::DEFAULT_INIT_TIMEOUT_MS
        });
        adapter
    }

    #[cfg(feature = "with_grpc")]
    fn create_grpc_adapter(
        sdk_key: &str,
        config: &SpecAdapterConfig,
        options: &StatsigOptions,
    ) -> Option<Arc<dyn SpecsAdapter>> {
        Some(Arc::new(StatsigGrpcSpecsAdapter::new(
            sdk_key,
            config,
            Some(options),
        )))
    }

    #[cfg(not(feature = "with_grpc"))]
    fn create_grpc_adapter(
        _sdk_key: &str,
        _config: &SpecAdapterConfig,
        _options: &StatsigOptions,
    ) -> Option<Arc<dyn SpecsAdapter>> {
        log_e!(
            TAG,
            "Trying to use grpc websocket adapter but with grpc feature is not enabled"
        );
        None
    }
}

#[async_trait]
impl SpecsAdapter for StatsigCustomizedSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        for adapter in &self.adapters {
            match adapter.to_owned().start(statsig_runtime).await {
                Ok(()) => {
                    return Ok(());
                }
                Err(e) => {
                    log_w!(
                        TAG,
                        "Failed to initialize from {} adapter: {e}",
                        adapter.get_type_name(),
                    );
                    // Carry on
                }
            }
        }
        return Err(StatsigErr::UnstartedAdapter(
            "Failed to start any adapters".to_string(),
        ));
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        for adapter in &self.adapters {
            adapter.initialize(listener.clone());
        }
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // TODO: we probably should have another option for config sync sources, but for now, we only have one
        for adapter in &self.adapters {
            match adapter
                .to_owned()
                .schedule_background_sync(statsig_runtime)
                .await
            {
                Ok(()) => return Ok(()),
                Err(_) => {
                    // Carry on and try  next adapter
                    log_i!(
                        TAG,
                        "Skipping schedule bg sync for {}",
                        adapter.get_type_name()
                    );
                }
            }
        }
        Ok(())
    }

    async fn schedule_background_sync_after_fork(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        for adapter in &self.adapters {
            match adapter
                .to_owned()
                .schedule_background_sync_after_fork(statsig_runtime)
                .await
            {
                Ok(()) => return Ok(()),
                Err(_) => {
                    log_i!(
                        TAG,
                        "Skipping post-fork bg sync for {}",
                        adapter.get_type_name()
                    );
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatsigOptions;

    fn network_config(init_timeout_ms: u64) -> SpecAdapterConfig {
        SpecAdapterConfig {
            adapter_type: SpecsAdapterType::NetworkHttp,
            init_timeout_ms,
            specs_url: None,
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            domain_name: None,
        }
    }

    #[test]
    fn explicit_per_adapter_timeout_overrides_global() {
        let options = StatsigOptions {
            init_timeout_ms: Some(30_000),
            ..StatsigOptions::default()
        };
        let adapter = StatsigCustomizedSpecsAdapter::http_adapter_from_config(
            "secret-key",
            &options,
            &network_config(10_000),
        );
        assert_eq!(adapter.init_timeout_ms(), 10_000);
    }

    #[test]
    fn zero_per_adapter_timeout_uses_default() {
        let options = StatsigOptions {
            init_timeout_ms: Some(30_000),
            ..StatsigOptions::default()
        };
        let adapter = StatsigCustomizedSpecsAdapter::http_adapter_from_config(
            "secret-key",
            &options,
            &network_config(0),
        );
        assert_eq!(adapter.init_timeout_ms(), crate::DEFAULT_INIT_TIMEOUT_MS);
    }
}
