use crate::networking::{NetworkClient, NetworkError, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_e, log_error_to_statsig_and_console, SpecsSource, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;
use tokio::time::sleep;

pub const DEFAULT_SPECS_URL: &str = "https://api.statsigcdn.com/v2/download_config_specs";
pub const DEFAULT_SYNC_INTERVAL_MS: u32 = 10_000;

const TAG: &str = stringify!(StatsigHttpSpecsAdapter);
pub struct StatsigHttpSpecsAdapter {
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    network: NetworkClient,
    specs_url: String,
    fallback_url: Option<String>,
    sync_interval_duration: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl StatsigHttpSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        specs_url: Option<&String>,
        fallback_to_statsig_api: bool,
        sync_interval: Option<u32>,
    ) -> Self {
        let fallback_url = if fallback_to_statsig_api {
            construct_fallback_specs_url(sdk_key, specs_url)
        } else {
            None
        };

        let headers = StatsigMetadata::get_constant_request_headers(sdk_key);

        Self {
            listener: RwLock::new(None),
            network: NetworkClient::new(sdk_key, Some(headers)),
            specs_url: construct_specs_url(sdk_key, specs_url),
            fallback_url,
            sync_interval_duration: Duration::from_millis(
                sync_interval.unwrap_or(DEFAULT_SYNC_INTERVAL_MS) as u64,
            ),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        }
    }

    pub async fn fetch_specs_from_network(
        &self,
        current_store_lcut: Option<u64>,
    ) -> Option<String> {
        let query_params =
            current_store_lcut.map(|lcut| HashMap::from([("sinceTime".into(), lcut.to_string())]));

        let request_args = RequestArgs {
            url: self.specs_url.clone(),
            retries: 2,
            query_params,
            accept_gzip_response: true,
            ..RequestArgs::new()
        };

        match self.network.get(request_args.clone()).await {
            Ok(response) => Some(response),
            Err(NetworkError::RetriesExhausted) => {self.handle_fallback_request(request_args).await},
            Err(_) => None,
        }
    }

    async fn handle_fallback_request(&self, mut request_args: RequestArgs) -> Option<String>{
        let fallback_url = match &self.fallback_url {
            Some(url) => url.clone(),
            None => return None,
        };

        request_args.url = fallback_url;

        // TODO logging

        match self.network.get(request_args).await {
            Ok(response) => Some(response),
            Err(_) => None,
        }
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => {
                log_e!(TAG, "No strong reference found");
                return;
            }
        };

        let lcut = match strong_self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info().lcut,
                None => None,
            },
            Err(_) => None,
        };

        if let Err(e) = strong_self.manually_sync_specs(lcut).await {
            log_e!(TAG, "Background specs sync failed: {}", e);
        }
    }

    async fn manually_sync_specs(&self, current_store_lcut: Option<u64>) -> Result<(), StatsigErr> {
        if let Ok(lock) = self.listener.read() {
            if lock.is_none() {
                return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()));
            }
        }

        let response = self.fetch_specs_from_network(current_store_lcut).await;

        let data = match response {
            Some(r) => r,
            None => {
                let msg = "No specs result from network";
                log_e!(TAG, "{}", msg);
                return Err(StatsigErr::NetworkError(msg.to_string()));
            }
        };

        let update = SpecsUpdate {
            data,
            source: SpecsSource::Network,
            received_at: Utc::now().timestamp_millis() as u64,
        };

        match self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.did_receive_specs_update(update),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => {
                log_error_to_statsig_and_console!(
                    &self.ops_stats,
                    TAG,
                    "Failed to acquire read lock on listener: {}",
                    e
                );
                Err(StatsigErr::LockFailure(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigHttpSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        let lcut = listener.get_current_specs_info().lcut;
        self.manually_sync_specs(lcut).await
    }


    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on listener: {}", e);
            }
        }
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let weak_self: Weak<StatsigHttpSpecsAdapter> = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration;

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

// only fallback when the spec_url is not the DEFAULT_SPECS_URL
fn construct_fallback_specs_url(sdk_key: &str, spec_url: Option<&String>) -> Option<String> {
    match spec_url {
        Some(u) if u != DEFAULT_SPECS_URL => Some(format!("{}/{}.json", u, sdk_key)),
        _ => None,
    }
}
