use crate::networking::{NetworkClient, RequestArgs};
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_e, unwrap_or_return_with, SpecsSource, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;
use tokio::time::sleep;

pub const DEFAULT_SPECS_URL: &str = "https://api.statsigcdn.com/v2/download_config_specs";
const DEFAULT_SYNC_INTERVAL_MS: u32 = 10_000;

struct AdapterContext {
    network: NetworkClient,
    listener: Arc<dyn SpecsUpdateListener>,
}

pub struct StatsigHttpSpecsAdapter {
    context: RwLock<Option<Arc<AdapterContext>>>,
    specs_url: String,
    sync_interval_duration: Duration,
    sdk_key: String,
}

impl StatsigHttpSpecsAdapter {
    pub fn new(sdk_key: &str, specs_url: Option<&String>, sync_interval: Option<u32>) -> Self {
        Self {
            context: RwLock::new(None),
            specs_url: construct_specs_url(sdk_key, specs_url),
            sync_interval_duration: Duration::from_millis(
                sync_interval.unwrap_or(DEFAULT_SYNC_INTERVAL_MS) as u64,
            ),
            sdk_key: sdk_key.to_string(),
        }
    }

    pub async fn fetch_specs_from_network(
        &self,
        current_store_lcut: Option<u64>,
    ) -> Option<String> {
        let context = unwrap_or_return_with!(self.get_context(), || {
            log_e!("No context found");
            return None;
        });

        self.fetch_specs_from_network_impl(&context, current_store_lcut)
            .await
    }

    async fn fetch_specs_from_network_impl(
        &self,
        context: &AdapterContext,
        current_store_lcut: Option<u64>,
    ) -> Option<String> {
        let query_params =
            current_store_lcut.map(|lcut| HashMap::from([("sinceTime".into(), lcut.to_string())]));

        context
            .network
            .get(RequestArgs {
                url: self.specs_url.clone(),
                retries: 2,
                query_params,
                accept_gzip_response: true,
                ..RequestArgs::new()
            })
            .await
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => {
                log_e!("StatsigHttpSpecsAdapter - No strong reference found");
                return;
            }
        };

        let context = unwrap_or_return_with!(strong_self.get_context(), || {
            log_e!("StatsigHttpSpecsAdapter - No context found");
            return;
        });

        let lcut = context.listener.get_current_specs_info().lcut;
        if let Err(e) = strong_self.manually_sync_specs(lcut).await {
            log_e!(
                "StatsigHttpSpecsAdapter - Background specs sync failed: {}",
                e
            );
        }
    }

    async fn manually_sync_specs(&self, current_store_lcut: Option<u64>) -> Result<(), StatsigErr> {
        let context = unwrap_or_return_with!(self.get_context(), || {
            log_e!("StatsigHttpSpecsAdapter - No context found");
            return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()));
        });

        let res = self
            .fetch_specs_from_network_impl(&context, current_store_lcut)
            .await;

        let data = match res {
            Some(r) => r,
            None => {
                let msg = "StatsigHttpSpecsAdapter - No specs result from network";
                log_e!("{}", msg);
                return Err(StatsigErr::NetworkError(msg.to_string()));
            }
        };

        let update = SpecsUpdate {
            data,
            source: SpecsSource::Network,
            received_at: Utc::now().timestamp_millis() as u64,
        };

        context.listener.did_receive_specs_update(update);

        Ok(())
    }

    fn get_context(&self) -> Option<Arc<AdapterContext>> {
        match self.context.read() {
            Ok(lock) => lock.as_ref().cloned(),
            Err(e) => {
                log_e!(
                    "StatsigHttpSpecsAdapter - Failed to acquire read lock on context: {}",
                    e
                );
                return None;
            }
        }
    }

    fn setup_context(
        &self,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener>,
    ) {
        let headers = StatsigMetadata::get_constant_request_headers(&self.sdk_key);
        let context = Arc::new(AdapterContext {
            network: NetworkClient::new(statsig_runtime, &self.sdk_key, Some(headers)),
            listener,
        });

        match self.context.write() {
            Ok(mut lock) => *lock = Some(context),
            Err(e) => {
                log_e!(
                    "StatsigHttpSpecsAdapter - Failed to acquire write lock on context: {}",
                    e
                );
            }
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigHttpSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        let lcut = listener.get_current_specs_info().lcut;
        self.setup_context(statsig_runtime, listener);
        self.manually_sync_specs(lcut).await
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let weak_self: Weak<StatsigHttpSpecsAdapter> = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration.clone();

        statsig_runtime.spawn("http_specs_bg_sync", move |shutdown_notify| async move {
            loop {
                tokio::select! {
                    _ = sleep(interval_duration) => {
                        Self::run_background_sync(&weak_self).await;
                    }
                    _ = shutdown_notify.notified() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigHttpSpecsAdapter).to_string()
    }
}

fn construct_specs_url(sdk_key: &str, spec_url: Option<&String>) -> String {
    let base = match spec_url {
        Some(u) => u,
        _ => DEFAULT_SPECS_URL,
    };

    format!("{}/{}.json", base, sdk_key)
}
