use crate::networking::{NetworkClient, NetworkError, RequestArgs, ResponseData};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::utils::get_api_from_url;
use crate::DEFAULT_INIT_TIMEOUT_MS;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, SpecsSource, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use percent_encoding::percent_encode;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

use super::SpecsInfo;

pub struct NetworkResponse {
    pub data: ResponseData,
    pub loggable_api: String,
    pub requested_deltas: bool,
}

pub const DEFAULT_SPECS_URL: &str = "https://api.statsigcdn.com/v2/download_config_specs";
pub const DEFAULT_SYNC_INTERVAL_MS: u32 = 10_000;

#[allow(unused)]
pub const INIT_DICT_ID: &str = "null";

const TAG: &str = stringify!(StatsigHttpSpecsAdapter);
const CONFIG_SYNC_OVERALL_LATENCY_METRIC: &str = "config_sync_overall.latency";
const CONFIG_SYNC_OVERALL_FORMAT_TAG: &str = "format";
const CONFIG_SYNC_OVERALL_SOURCE_API_TAG: &str = "source_api";
const CONFIG_SYNC_OVERALL_ERROR_TAG: &str = "error";
const CONFIG_SYNC_OVERALL_NETWORK_SUCCESS_TAG: &str = "network_success";
const CONFIG_SYNC_OVERALL_PROCESS_SUCCESS_TAG: &str = "process_success";

pub struct StatsigHttpSpecsAdapter {
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    network: NetworkClient,
    sdk_key: String,
    specs_url: String,
    fallback_url: Option<String>,
    init_timeout_ms: u64,
    sync_interval_duration: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
    shutdown_notify: Arc<Notify>,
    allow_dcs_deltas: bool,
    use_deltas_next_request: AtomicBool,
}

// OB client -- START
// These types are only for the config_sync_overall.latency observability metric added in this change.
enum ResponseFormat {
    Json,
    PlainText,
    Protobuf,
    Unknown,
}

enum NetworkSyncOutcome {
    Success,
    Failure,
}

impl NetworkSyncOutcome {
    fn as_bool(&self) -> bool {
        matches!(self, Self::Success)
    }
}

impl ResponseFormat {
    fn as_str(&self) -> &str {
        match self {
            ResponseFormat::Json => "json",
            ResponseFormat::PlainText => "plain_text",
            ResponseFormat::Protobuf => "protobuf",
            ResponseFormat::Unknown => "unknown",
        }
    }
}
// OB client -- END

impl StatsigHttpSpecsAdapter {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        options: Option<&StatsigOptions>,
        override_url: Option<String>,
    ) -> Self {
        let default_options = StatsigOptions::default();
        let options_ref = options.unwrap_or(&default_options);

        let init_timeout_ms = options_ref
            .init_timeout_ms
            .unwrap_or(DEFAULT_INIT_TIMEOUT_MS);

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

        let headers = StatsigMetadata::get_constant_request_headers(
            sdk_key,
            options_ref.service_name.as_deref(),
        );
        let enable_dcs_deltas = options_ref.enable_dcs_deltas.unwrap_or(false);

        Self {
            listener: RwLock::new(None),
            network: NetworkClient::new(sdk_key, Some(headers), Some(options_ref)),
            sdk_key: sdk_key.to_string(),
            specs_url,
            fallback_url,
            init_timeout_ms,
            sync_interval_duration: Duration::from_millis(u64::from(
                options_ref
                    .specs_sync_interval_ms
                    .unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            )),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            shutdown_notify: Arc::new(Notify::new()),
            allow_dcs_deltas: enable_dcs_deltas,
            use_deltas_next_request: AtomicBool::new(enable_dcs_deltas),
        }
    }

    pub fn force_shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    pub async fn fetch_specs_from_network(
        &self,
        current_specs_info: SpecsInfo,
        trigger: SpecsSyncTrigger,
    ) -> Result<NetworkResponse, NetworkError> {
        let request_args = self.get_request_args(&current_specs_info, trigger);
        let url = request_args.url.clone();
        let requested_deltas = request_args.deltas_enabled;
        match self.handle_specs_request(request_args).await {
            Ok(response) => Ok(NetworkResponse {
                data: response,
                loggable_api: get_api_from_url(&url),
                requested_deltas,
            }),
            Err(e) => Err(e),
        }
    }

    fn get_request_args(
        &self,
        current_specs_info: &SpecsInfo,
        trigger: SpecsSyncTrigger,
    ) -> RequestArgs {
        let mut params = HashMap::new();

        params.insert("supports_proto".to_string(), "true".to_string());
        let headers = Some(HashMap::from([
            ("statsig-supports-proto".to_string(), "true".to_string()),
            (
                "accept-encoding".to_string(),
                "statsig-br, gzip, deflate, br".to_string(),
            ),
        ]));

        if let Some(lcut) = current_specs_info.lcut {
            if lcut > 0 {
                params.insert("sinceTime".to_string(), lcut.to_string());
            }
        }

        let is_init_request = trigger == SpecsSyncTrigger::Initial;

        let timeout_ms = if is_init_request && self.init_timeout_ms > 0 {
            self.init_timeout_ms
        } else {
            0
        };

        if let Some(cs) = &current_specs_info.checksum {
            params.insert(
                "checksum".to_string(),
                percent_encode(cs.as_bytes(), percent_encoding::NON_ALPHANUMERIC).to_string(),
            );
        }

        let use_deltas_next_req = self.use_deltas_next_request.load(Ordering::SeqCst);
        if use_deltas_next_req {
            params.insert("accept_deltas".to_string(), "true".to_string());
        }

        RequestArgs {
            url: construct_specs_url(self.specs_url.as_str(), self.sdk_key.as_str()),
            retries: match trigger {
                SpecsSyncTrigger::Initial | SpecsSyncTrigger::Manual => 0,
                SpecsSyncTrigger::Background => 3,
            },
            query_params: Some(params),
            deltas_enabled: use_deltas_next_req,
            accept_gzip_response: true,
            diagnostics_key: Some(KeyType::DownloadConfigSpecs),
            timeout_ms,
            headers,
            ..RequestArgs::new()
        }
    }

    async fn handle_fallback_request(
        &self,
        mut request_args: RequestArgs,
    ) -> Result<NetworkResponse, NetworkError> {
        let requested_deltas = request_args.deltas_enabled;
        let fallback_url = match &self.fallback_url {
            Some(url) => construct_specs_url(url.as_str(), &self.sdk_key),
            None => {
                return Err(NetworkError::RequestFailed(
                    request_args.url.clone(),
                    None,
                    "No fallback URL".to_string(),
                ))
            }
        };

        request_args.url = fallback_url.clone();

        // TODO logging

        let response = self.handle_specs_request(request_args).await?;
        Ok(NetworkResponse {
            data: response,
            loggable_api: get_api_from_url(&fallback_url),
            requested_deltas,
        })
    }

    async fn handle_specs_request(
        &self,
        request_args: RequestArgs,
    ) -> Result<ResponseData, NetworkError> {
        let url = request_args.url.clone();
        let response = self.network.get(request_args).await?;
        match response.data {
            Some(data) => Ok(data),
            None => Err(NetworkError::RequestFailed(
                url,
                None,
                response.error.unwrap_or("No data in response".to_string()),
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
        if let Err(e) = self
            .manually_sync_specs(specs_info, SpecsSyncTrigger::Background)
            .await
        {
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

    async fn manually_sync_specs(
        &self,
        current_specs_info: SpecsInfo,
        trigger: SpecsSyncTrigger,
    ) -> Result<(), StatsigErr> {
        if let Some(lock) = self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            if lock.is_none() {
                return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()));
            }
        }

        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        let response = self
            .fetch_specs_from_network(current_specs_info.clone(), trigger)
            .await;
        let (mut source_api, mut response_format, mut network_success) = match &response {
            Ok(response) => (
                response.loggable_api.clone(),
                Self::get_response_format(&response.data),
                NetworkSyncOutcome::Success,
            ),
            Err(_) => (
                get_api_from_url(&construct_specs_url(
                    self.specs_url.as_str(),
                    self.sdk_key.as_str(),
                )),
                ResponseFormat::Unknown,
                NetworkSyncOutcome::Failure,
            ),
        };

        let mut result = self.process_spec_data(response).await;

        if result.is_err() && self.fallback_url.is_some() {
            log_d!(TAG, "Falling back to statsig api");
            let response = self
                .handle_fallback_request(self.get_request_args(&current_specs_info, trigger))
                .await;
            match &response {
                Ok(response) => {
                    source_api = response.loggable_api.clone();
                    response_format = Self::get_response_format(&response.data);
                    network_success = NetworkSyncOutcome::Success;
                }
                Err(_) => {
                    // Backup request failed, so no successful network payload was returned.
                    if let Some(fallback_url) = self.fallback_url.as_ref() {
                        source_api = get_api_from_url(&construct_specs_url(
                            fallback_url.as_str(),
                            self.sdk_key.as_str(),
                        ));
                    }
                    network_success = NetworkSyncOutcome::Failure;
                }
            }
            result = self.process_spec_data(response).await;
        }

        let process_success = !matches!(result.as_ref(), Err(StatsigErr::NetworkError(_)));
        self.log_config_sync_overall_latency(
            sync_start_ms,
            &source_api,
            response_format.as_str(),
            network_success.as_bool(),
            process_success,
            result
                .as_ref()
                .err()
                .map_or_else(String::new, |e| e.to_string()),
        );

        result
    }

    // --------- START - Observability helpers ---------
    fn get_response_format(response_data: &ResponseData) -> ResponseFormat {
        if Self::is_response_protobuf(response_data) {
            return ResponseFormat::Protobuf;
        }

        let content_type = match response_data.get_header_ref("content-type") {
            Some(content_type) => content_type.to_ascii_lowercase(),
            None => return ResponseFormat::Unknown,
        };

        if content_type.contains("application/json") || content_type.contains("+json") {
            return ResponseFormat::Json;
        }

        if content_type.contains("text/plain") {
            return ResponseFormat::PlainText;
        }

        ResponseFormat::Unknown
    }

    fn is_response_protobuf(response_data: &ResponseData) -> bool {
        let content_type = response_data.get_header_ref("content-type");
        if content_type.map(|s| s.as_str().contains("application/octet-stream")) != Some(true) {
            return false;
        }

        let content_encoding = response_data.get_header_ref("content-encoding");
        content_encoding.map(|s| s.as_str().contains("statsig-br")) == Some(true)
    }

    fn log_config_sync_overall_latency(
        &self,
        sync_start_ms: u64,
        source_api: &str,
        response_format: &str,
        network_success: bool,
        process_success: bool,
        error: String,
    ) {
        let latency_ms =
            (Utc::now().timestamp_millis() as u64).saturating_sub(sync_start_ms) as f64;
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            CONFIG_SYNC_OVERALL_LATENCY_METRIC.to_string(),
            latency_ms,
            Some(HashMap::from([
                (
                    CONFIG_SYNC_OVERALL_SOURCE_API_TAG.to_string(),
                    get_api_from_url(source_api),
                ),
                (
                    CONFIG_SYNC_OVERALL_FORMAT_TAG.to_string(),
                    response_format.to_string(),
                ),
                (CONFIG_SYNC_OVERALL_ERROR_TAG.to_string(), error),
                (
                    CONFIG_SYNC_OVERALL_NETWORK_SUCCESS_TAG.to_string(),
                    network_success.to_string(),
                ),
                (
                    CONFIG_SYNC_OVERALL_PROCESS_SUCCESS_TAG.to_string(),
                    process_success.to_string(),
                ),
            ])),
        ));
    }
    // --------- END - Observability helpers ---------

    async fn process_spec_data(
        &self,
        response: Result<NetworkResponse, NetworkError>,
    ) -> Result<(), StatsigErr> {
        let resp = response.map_err(StatsigErr::NetworkError)?;
        let requested_deltas = resp.requested_deltas;

        let update = SpecsUpdate {
            data: resp.data,
            source: SpecsSource::Network,
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some(resp.loggable_api),
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

        if matches!(&result, Err(StatsigErr::ChecksumFailure(_))) {
            let was_deltas_used = self.use_deltas_next_request.swap(false, Ordering::SeqCst);
            if was_deltas_used {
                log_d!(TAG, "Disabling delta requests after checksum failure");
            }
        } else if result.is_ok() && !requested_deltas && self.allow_dcs_deltas {
            let was_deltas_used = self.use_deltas_next_request.swap(true, Ordering::SeqCst);
            if !was_deltas_used {
                log_d!(
                    TAG,
                    "Re-enabling delta requests after successful non-delta specs update"
                );
            }
        }

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
        self.manually_sync_specs(specs_info, SpecsSyncTrigger::Initial)
            .await
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
fn construct_specs_url(spec_url: &str, sdk_key: &str) -> String {
    format!("{spec_url}/{sdk_key}.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecsSyncTrigger {
    Initial,
    Background,
    Manual,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{networking::ResponseData, specs_adapter::SpecsUpdate, StatsigOptions};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    struct ChecksumFailingListener;

    impl SpecsUpdateListener for ChecksumFailingListener {
        fn did_receive_specs_update(&self, _update: SpecsUpdate) -> Result<(), StatsigErr> {
            Err(StatsigErr::ChecksumFailure(
                "simulated checksum failure".to_string(),
            ))
        }

        fn get_current_specs_info(&self) -> SpecsInfo {
            SpecsInfo::empty()
        }
    }

    struct ChecksumFailingThenSuccessListener {
        calls: AtomicUsize,
    }

    impl SpecsUpdateListener for ChecksumFailingThenSuccessListener {
        fn did_receive_specs_update(&self, _update: SpecsUpdate) -> Result<(), StatsigErr> {
            let curr = self.calls.fetch_add(1, Ordering::SeqCst);
            if curr == 0 {
                Err(StatsigErr::ChecksumFailure(
                    "simulated checksum failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        fn get_current_specs_info(&self) -> SpecsInfo {
            SpecsInfo::empty()
        }
    }

    #[tokio::test]
    async fn test_disable_accept_deltas_after_checksum_failure() {
        let options = StatsigOptions {
            enable_dcs_deltas: Some(true),
            ..StatsigOptions::default()
        };
        let adapter = StatsigHttpSpecsAdapter::new(
            "secret-key",
            Some(&options),
            Some("https://example.com/v2/download_config_specs".to_string()),
        );
        let specs_info = SpecsInfo::empty();

        let request_before = adapter.get_request_args(&specs_info, SpecsSyncTrigger::Manual);
        assert_eq!(
            request_before
                .query_params
                .as_ref()
                .and_then(|p| p.get("accept_deltas"))
                .map(String::as_str),
            Some("true")
        );

        adapter.initialize(Arc::new(ChecksumFailingListener));
        let result = adapter
            .process_spec_data(Ok(NetworkResponse {
                data: ResponseData::from_bytes(vec![]),
                loggable_api: "test-api".to_string(),
                requested_deltas: true,
            }))
            .await;

        assert!(matches!(result, Err(StatsigErr::ChecksumFailure(_))));

        let request_after = adapter.get_request_args(&specs_info, SpecsSyncTrigger::Manual);
        assert!(request_after
            .query_params
            .as_ref()
            .is_none_or(|p| !p.contains_key("accept_deltas")));
    }

    #[tokio::test]
    async fn test_reenable_accept_deltas_after_successful_non_delta_update() {
        let options = StatsigOptions {
            enable_dcs_deltas: Some(true),
            ..StatsigOptions::default()
        };
        let adapter = StatsigHttpSpecsAdapter::new(
            "secret-key",
            Some(&options),
            Some("https://example.com/v2/download_config_specs".to_string()),
        );
        let specs_info = SpecsInfo::empty();

        adapter.initialize(Arc::new(ChecksumFailingThenSuccessListener {
            calls: AtomicUsize::new(0),
        }));

        let first_result = adapter
            .process_spec_data(Ok(NetworkResponse {
                data: ResponseData::from_bytes(vec![]),
                loggable_api: "test-api".to_string(),
                requested_deltas: true,
            }))
            .await;

        assert!(matches!(first_result, Err(StatsigErr::ChecksumFailure(_))));

        let request_after_failure = adapter.get_request_args(&specs_info, SpecsSyncTrigger::Manual);
        assert!(request_after_failure
            .query_params
            .as_ref()
            .is_none_or(|p| !p.contains_key("accept_deltas")));

        let second_result = adapter
            .process_spec_data(Ok(NetworkResponse {
                data: ResponseData::from_bytes(vec![]),
                loggable_api: "test-api".to_string(),
                requested_deltas: false,
            }))
            .await;

        assert!(second_result.is_ok());

        let request_after_success = adapter.get_request_args(&specs_info, SpecsSyncTrigger::Manual);
        assert_eq!(
            request_after_success
                .query_params
                .as_ref()
                .and_then(|p| p.get("accept_deltas"))
                .map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn test_get_response_format_json() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let data = ResponseData::from_bytes_with_headers(vec![], Some(headers));
        assert!(matches!(
            StatsigHttpSpecsAdapter::get_response_format(&data),
            ResponseFormat::Json
        ));
    }

    #[test]
    fn test_get_response_format_plain_text() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        let data = ResponseData::from_bytes_with_headers(vec![], Some(headers));
        assert!(matches!(
            StatsigHttpSpecsAdapter::get_response_format(&data),
            ResponseFormat::PlainText
        ));
    }

    #[test]
    fn test_get_response_format_protobuf() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("content-encoding".to_string(), "statsig-br".to_string());
        let data = ResponseData::from_bytes_with_headers(vec![], Some(headers));
        assert!(matches!(
            StatsigHttpSpecsAdapter::get_response_format(&data),
            ResponseFormat::Protobuf
        ));
    }

    #[test]
    fn test_get_response_format_unknown_without_content_type() {
        let data = ResponseData::from_bytes(vec![]);
        assert!(matches!(
            StatsigHttpSpecsAdapter::get_response_format(&data),
            ResponseFormat::Unknown
        ));
    }
}
