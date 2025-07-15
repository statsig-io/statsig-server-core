use crate::networking::{NetworkClient, NetworkError, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::utils::get_api_from_url;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, SpecsSource, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use percent_encoding::percent_encode;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

use super::SpecsInfo;

pub struct NetworkResponse {
    pub data: Vec<u8>,
    pub api: String,
}

pub const DEFAULT_SPECS_URL: &str = "https://api.statsigcdn.com/v2/download_config_specs";
pub const DEFAULT_SYNC_INTERVAL_MS: u32 = 10_000;
#[allow(unused)]
pub const INIT_DICT_ID: &str = "null";

const TAG: &str = stringify!(StatsigHttpSpecsAdapter);
pub struct StatsigHttpSpecsAdapter {
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    network: NetworkClient,
    sdk_key: String,
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
        options: Option<&StatsigOptions>,
        override_url: Option<String>,
    ) -> Self {
        let default_options = StatsigOptions::default();
        let options_ref = options.unwrap_or(&default_options);

        let specs_url = match override_url {
            Some(url) => url,
            None => options_ref
                .specs_url
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or(DEFAULT_SPECS_URL.to_string()),
        };

        // only fallback when the spec_url is not the DEFAULT_SPECS_URL
        let fallback_url = if options_ref.fallback_to_statsig_api.unwrap_or(false)
            && specs_url != DEFAULT_SPECS_URL
        {
            Some(DEFAULT_SPECS_URL.to_string())
        } else {
            None
        };

        let headers = StatsigMetadata::get_constant_request_headers(sdk_key);

        Self {
            listener: RwLock::new(None),
            network: NetworkClient::new(sdk_key, Some(headers), Some(options_ref)),
            sdk_key: sdk_key.to_string(),
            specs_url,
            fallback_url,
            sync_interval_duration: Duration::from_millis(u64::from(
                options_ref
                    .specs_sync_interval_ms
                    .unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            )),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    pub fn force_shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    pub async fn fetch_specs_from_network(
        &self,
        current_specs_info: SpecsInfo,
    ) -> Result<NetworkResponse, NetworkError> {
        let request_args = self.get_request_args(&current_specs_info);
        let url = request_args.url.clone();
        match self.handle_specs_request(request_args).await {
            Ok(response) => Ok(NetworkResponse {
                data: response,
                api: get_api_from_url(&url),
            }),
            Err(e) => Err(e),
        }
    }

    fn get_request_args(&self, current_specs_info: &SpecsInfo) -> RequestArgs {
        let mut params = HashMap::new();
        if let Some(lcut) = current_specs_info.lcut {
            params.insert("sinceTime".to_string(), lcut.to_string());
        }
        if let Some(cs) = &current_specs_info.checksum {
            params.insert(
                "checksum".to_string(),
                percent_encode(cs.as_bytes(), percent_encoding::NON_ALPHANUMERIC).to_string(),
            );
        }

        RequestArgs {
            url: construct_specs_url(
                self.specs_url.as_str(),
                self.sdk_key.as_str(),
                current_specs_info.zstd_dict_id.as_deref(),
            ),
            query_params: Some(params),
            accept_gzip_response: true,
            diagnostics_key: Some(KeyType::DownloadConfigSpecs),
            ..RequestArgs::new()
        }
    }

    async fn handle_fallback_request(
        &self,
        mut request_args: RequestArgs,
        current_specs_info: SpecsInfo,
    ) -> Result<NetworkResponse, NetworkError> {
        let fallback_url = match &self.fallback_url {
            Some(url) => construct_specs_url(
                url.as_str(),
                &self.sdk_key,
                current_specs_info.zstd_dict_id.as_deref(),
            ),
            None => {
                return Err(NetworkError::RequestFailed(
                    request_args.url.clone(),
                    0,
                    "No fallback URL".to_string(),
                ))
            }
        };

        request_args.url = fallback_url.clone();

        // TODO logging

        let response = self.handle_specs_request(request_args).await?;
        Ok(NetworkResponse {
            data: response,
            api: get_api_from_url(&fallback_url),
        })
    }

    // TODO: return a decompressed and parsed SpecsResponse
    async fn handle_specs_request(
        &self,
        request_args: RequestArgs,
    ) -> Result<Vec<u8>, NetworkError> {
        let url = request_args.url.clone();
        let response = self.network.get(request_args).await?;
        match response.data {
            Some(data) => Ok(data),
            None => Err(NetworkError::RequestFailed(
                url,
                0,
                "No data in response".to_string(),
            )),
        }
    }

    pub async fn run_background_sync(self: Arc<Self>) {
        let specs_info = match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info(),
                None => SpecsInfo::empty(),
            },
            None => SpecsInfo::error(),
        };

        self.ops_stats
            .set_diagnostics_context(ContextType::ConfigSync);
        if let Err(e) = self.manually_sync_specs(specs_info).await {
            if let StatsigErr::NetworkError(NetworkError::DisableNetworkOn(_)) = e {
                return;
            }
            log_e!(TAG, "Background specs sync failed: {}", e);
        }
        self.ops_stats.enqueue_diagnostics_event(
            Some(KeyType::DownloadConfigSpecs),
            Some(ContextType::ConfigSync),
        );
    }

    async fn manually_sync_specs(&self, current_specs_info: SpecsInfo) -> Result<(), StatsigErr> {
        if let Some(lock) = self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
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
            let response = self
                .handle_fallback_request(
                    self.get_request_args(&current_specs_info),
                    current_specs_info,
                )
                .await;
            return self.process_spec_data(response).await;
        }

        result
    }

    async fn process_spec_data(
        &self,
        response: Result<NetworkResponse, NetworkError>,
    ) -> Result<(), StatsigErr> {
        let resp = response.map_err(StatsigErr::NetworkError)?;

        let update = SpecsUpdate {
            data: resp.data,
            source: SpecsSource::Network,
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some(resp.api),
        };

        self.ops_stats.add_marker(
            Marker::new(
                KeyType::DownloadConfigSpecs,
                ActionType::Start,
                Some(StepType::Process),
            ),
            None,
        );

        let result = match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => listener.did_receive_specs_update(update),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            None => {
                let err =
                    StatsigErr::LockFailure("Failed to acquire read lock on listener".to_string());
                log_error_to_statsig_and_console!(&self.ops_stats, TAG, err.clone());
                Err(err)
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
        let specs_info = match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info(),
                None => SpecsInfo::empty(),
            },
            None => SpecsInfo::error(),
        };
        self.manually_sync_specs(specs_info).await
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self
            .listener
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut lock) => *lock = Some(listener),
            None => {
                log_e!(TAG, "Failed to acquire write lock on listener");
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
                        if let Some(strong_self) = weak_self.upgrade() {
                            Self::run_background_sync(strong_self).await;
                        } else {
                            log_e!(TAG, "Strong reference to StatsigHttpSpecsAdapter lost. Stopping background sync");
                            break;
                        }
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
        })?;

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

#[allow(unused)]
fn construct_specs_url(spec_url: &str, sdk_key: &str, dict_id: Option<&str>) -> String {
    #[cfg(feature = "with_shared_dict_compression")]
    {
        let dict_id = dict_id.unwrap_or(INIT_DICT_ID);
        format!("{spec_url}/d/{dict_id}/{sdk_key}.json")
    }
    #[cfg(not(feature = "with_shared_dict_compression"))]
    format!("{spec_url}/{sdk_key}.json")
}
