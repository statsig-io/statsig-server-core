use crate::networking::{NetworkClient, NetworkError, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_d, log_e, log_error_to_statsig_and_console, SpecsSource, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use percent_encoding::percent_encode;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

use super::SpecsInfo;

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
    shutdown_notify: Arc<Notify>,
}

impl StatsigHttpSpecsAdapter {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        specs_url: Option<&String>,
        fallback_to_statsig_api: bool,
        sync_interval: Option<u32>,
        disable_network: Option<bool>,
    ) -> Self {
        let fallback_url = if fallback_to_statsig_api {
            construct_fallback_specs_url(sdk_key, specs_url)
        } else {
            None
        };

        let headers = StatsigMetadata::get_constant_request_headers(sdk_key);

        Self {
            listener: RwLock::new(None),
            network: NetworkClient::new(sdk_key, Some(headers), disable_network),
            specs_url: construct_specs_url(sdk_key, specs_url),
            fallback_url,
            sync_interval_duration: Duration::from_millis(u64::from(
                sync_interval.unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            )),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn fetch_specs_from_network(
        &self,
        current_specs_info: SpecsInfo,
    ) -> Result<String, NetworkError> {
        let request_args = self.get_request_args(current_specs_info);
        match self.handle_specs_request(request_args).await {
            Ok(response) => Ok(response),
            Err(e) => Err(e),
        }
    }

    fn get_request_args(&self, current_specs_info: SpecsInfo) -> RequestArgs {
        let mut params = HashMap::new();
        if let Some(lcut) = current_specs_info.lcut {
            params.insert("sinceTime".to_string(), lcut.to_string());
        }
        if let Some(cs) = current_specs_info.checksum {
            params.insert(
                "checksum".to_string(),
                percent_encode(cs.as_bytes(), percent_encoding::NON_ALPHANUMERIC).to_string(),
            );
        }

        RequestArgs {
            url: self.specs_url.clone(),
            query_params: Some(params),
            accept_gzip_response: true,
            diagnostics_key: Some(KeyType::DownloadConfigSpecs),
            ..RequestArgs::new()
        }
    }

    async fn handle_fallback_request(
        &self,
        mut request_args: RequestArgs,
    ) -> Result<String, NetworkError> {
        let fallback_url = match &self.fallback_url {
            Some(url) => url.clone(),
            None => return Err(NetworkError::RequestFailed),
        };

        request_args.url = fallback_url;

        // TODO logging

        self.handle_specs_request(request_args).await
    }

    // TODO: return a decompressed and parsed SpecsResponse
    async fn handle_specs_request(
        &self,
        request_args: RequestArgs,
    ) -> Result<String, NetworkError> {
        let response = self.network.get(request_args).await?;
        match response.data {
            Some(data) => Ok(String::from_utf8(data)
                .map_err(|e| NetworkError::SerializationError(e.to_string()))?),
            None => Err(NetworkError::RequestFailed),
        }
    }

    pub async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = if let Some(s) = weak_self.upgrade() {
            s
        } else {
            log_e!(TAG, "No strong reference found");
            return;
        };

        let specs_info = match strong_self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info(),
                None => SpecsInfo::empty(),
            },
            Err(_) => SpecsInfo::error(),
        };

        strong_self
            .ops_stats
            .set_diagnostics_context(ContextType::ConfigSync);
        if let Err(e) = strong_self.manually_sync_specs(specs_info).await {
            if let StatsigErr::NetworkError(NetworkError::DisableNetworkOn, _) = e {
                return;
            }
            log_e!(TAG, "Background specs sync failed: {}", e);
        }
        strong_self.ops_stats.enqueue_diagnostics_event(
            Some(KeyType::DownloadConfigSpecs),
            Some(ContextType::ConfigSync),
        );
    }

    async fn manually_sync_specs(&self, current_specs_info: SpecsInfo) -> Result<(), StatsigErr> {
        if let Ok(lock) = self.listener.read() {
            if lock.is_none() {
                return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()));
            }
        }

        let response = self
            .fetch_specs_from_network(current_specs_info.clone())
            .await;
        let result = self.process_spec_data(response).await;

        if result.is_err() && self.fallback_url.is_some() {
            log_d!(TAG, "Falling back to statsig api");
            let request_args = self.get_request_args(current_specs_info);
            let response = self.handle_fallback_request(request_args).await;
            return self.process_spec_data(response).await;
        }

        result
    }

    async fn process_spec_data(
        &self,
        response: Result<String, NetworkError>,
    ) -> Result<(), StatsigErr> {
        let data = response.map_err(|e| {
            let msg = "No specs result from network";
            StatsigErr::NetworkError(e, Some(msg.to_string()))
        })?;

        let update = SpecsUpdate {
            data,
            source: SpecsSource::Network,
            received_at: Utc::now().timestamp_millis() as u64,
        };

        self.ops_stats.add_marker(
            Marker::new(
                KeyType::DownloadConfigSpecs,
                ActionType::Start,
                Some(StepType::Process),
            ),
            None,
        );

        let result = match self.listener.read() {
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
        };

        self.ops_stats.add_marker(
            Marker::new(
                KeyType::DownloadConfigSpecs,
                ActionType::End,
                Some(StepType::Process),
            )
            .with_is_success(result.is_ok()),
            None,
        );

        result
    }
}

#[async_trait]
impl SpecsAdapter for StatsigHttpSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let specs_info = match self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info(),
                None => SpecsInfo::empty(),
            },
            Err(_) => SpecsInfo::error(),
        };
        self.manually_sync_specs(specs_info).await
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on listener: {}", e);
            }
        }
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let weak_self: Weak<StatsigHttpSpecsAdapter> = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration;
        let shutdown_notify = self.shutdown_notify.clone();

        statsig_runtime.spawn("http_specs_bg_sync", move |rt_shutdown_notify| async move {
            loop {
                tokio::select! {
                    () = sleep(interval_duration) => {
                        Self::run_background_sync(&weak_self).await;
                    }
                    () = rt_shutdown_notify.notified() => {
                        log_d!(TAG, "Runtime shutdown. Shutting down specs background sync");
                        break;
                    },
                    () = shutdown_notify.notified() => {
                        log_d!(TAG, "Shutting down specs background sync");
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
        self.shutdown_notify.notify_one();
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

    format!("{base}/{sdk_key}.json")
}

// only fallback when the spec_url is not the DEFAULT_SPECS_URL
fn construct_fallback_specs_url(sdk_key: &str, spec_url: Option<&String>) -> Option<String> {
    match spec_url {
        Some(u) if u != DEFAULT_SPECS_URL => Some(format!("{DEFAULT_SPECS_URL}/{sdk_key}.json")),
        _ => None,
    }
}
