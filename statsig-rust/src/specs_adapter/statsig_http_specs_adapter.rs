use super::config_spec_background_sync_metrics::log_config_sync_overall_latency;
use super::response_format::{get_specs_response_format, SpecsResponseFormat};
use crate::networking::{NetworkClient, NetworkError, RequestArgs, ResponseData};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::specs_adapter::{SpecsAdapter, SpecsUpdate, SpecsUpdateListener};
use crate::specs_response::spec_types::SpecsResponseNoUpdates;
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
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

/// Result of applying a fetched specs payload to the listener, plus whether the
/// response actually carried updates (`has_updates: true`). The latter lets the
/// stalled-delta recovery distinguish a wedged stream (updates that never
/// advance the LCUT) from an idle config (`has_updates: false`).
struct ProcessSpecDataOutcome {
    result: Result<(), StatsigErr>,
    had_updates: bool,
}

pub const DEFAULT_SPECS_URL: &str = "https://api.statsigcdn.com/v2/download_config_specs";
pub const DEFAULT_SYNC_INTERVAL_MS: u32 = 10_000;

#[allow(unused)]
pub const INIT_DICT_ID: &str = "null";

const TAG: &str = stringify!(StatsigHttpSpecsAdapter);
const STATSIG_NETWORK_FALLBACK_THRESHOLD: u32 = 5;

/// Default number of consecutive background delta syncs that received updates
/// but made no LCUT progress before the adapter forces a single full (non-delta)
/// resync. Used when [`crate::StatsigOptions::dcs_delta_no_progress_threshold`]
/// is `None`. Set that option to `Some(0)` to disable the recovery entirely.
pub const DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD: u32 = 10;

/// Minimum wall-clock gap between forced full resyncs. A stalled delta stream is
/// recovered by forcing a full sync, but we throttle how often that can happen so
/// a genuinely idle config (which also never advances its LCUT) does not trigger
/// repeated full downloads. Being time-based (rather than keyed on the stalled
/// LCUT) guarantees recovery re-arms on its own even if the stream stays wedged
/// at the same LCUT indefinitely.
const MIN_FORCED_FULL_RESYNC_INTERVAL_MS: u64 = 5 * 60 * 1000;

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
    background_sync_failure_count: AtomicU32,
    // Consecutive background delta syncs that received deltas but did not advance
    // the LCUT.
    delta_no_progress_count: AtomicU32,
    // Number of consecutive no-progress delta syncs that force a full resync.
    delta_no_progress_threshold: u32,
    // Unix-epoch millis of the last forced full resync (0 = never), used to
    // throttle how often a stalled delta stream forces a full download.
    last_forced_full_resync_at_ms: AtomicU64,
}

// OB client -- START
// These types are only for the config_sync_overall.latency observability metric added in this change.

enum NetworkSyncOutcome {
    Success,
    Failure,
}

impl NetworkSyncOutcome {
    fn as_bool(&self) -> bool {
        matches!(self, Self::Success)
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
        // `None` → default threshold (recovery on). `Some(0)` → disabled.
        // `Some(n)` for n > 0 → custom threshold.
        let delta_no_progress_threshold = match options_ref.dcs_delta_no_progress_threshold {
            None => DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD,
            Some(threshold) => threshold,
        };

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
            background_sync_failure_count: AtomicU32::new(0),
            delta_no_progress_count: AtomicU32::new(0),
            delta_no_progress_threshold,
            last_forced_full_resync_at_ms: AtomicU64::new(0),
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

    fn should_attempt_fallback(
        &self,
        trigger: SpecsSyncTrigger,
        result: &Result<(), StatsigErr>,
    ) -> bool {
        if result.is_ok() || self.fallback_url.is_none() {
            return false;
        }

        if trigger != SpecsSyncTrigger::Background {
            return true;
        }

        let failure_count = self
            .background_sync_failure_count
            .fetch_add(1, Ordering::SeqCst)
            + 1;

        if failure_count.is_multiple_of(STATSIG_NETWORK_FALLBACK_THRESHOLD) {
            return true;
        }

        log_d!(
            TAG,
            "Skipping fallback on background sync failure {}. Retrying fallback every {} failures.",
            failure_count,
            STATSIG_NETWORK_FALLBACK_THRESHOLD
        );

        false
    }

    /// Reads the current specs info from the listener under a bounded read lock.
    /// Returns `SpecsInfo::empty()` when no listener is set and
    /// `SpecsInfo::error()` when the lock can't be acquired within the timeout.
    fn read_current_specs_info(&self) -> SpecsInfo {
        match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info(),
                None => SpecsInfo::empty(),
            },
            None => SpecsInfo::error(),
        }
    }

    fn get_listener_lcut(&self) -> Option<u64> {
        self.read_current_specs_info().lcut
    }

    /// Recovers from a wedged delta stream: a client that keeps receiving deltas
    /// it cannot apply (the server has newer data, but the in-memory LCUT never
    /// advances). Counts consecutive background delta syncs that carried updates
    /// yet made no LCUT progress and, once the streak reaches
    /// `delta_no_progress_threshold`, forces the next request to be a full
    /// (non-delta) resync. This covers both silently-dropped deltas and
    /// non-checksum apply errors (protobuf/json parse, lock failure) without
    /// waiting for an incidental full refresh. A successful non-delta update
    /// re-enables deltas via `process_spec_data`.
    ///
    /// A "no updates" response (`has_updates: false`) means the client is already
    /// current, so it is treated as progress, not a stall — an idle config never
    /// triggers a forced resync.
    ///
    /// Forced resyncs are throttled by wall-clock time
    /// (`MIN_FORCED_FULL_RESYNC_INTERVAL_MS`) so a genuinely wedged stream still
    /// re-arms on its own if it stays stuck at the same LCUT indefinitely.
    fn maybe_force_full_resync_on_stalled_deltas(
        &self,
        trigger: SpecsSyncTrigger,
        pre_lcut: Option<u64>,
        requested_deltas: bool,
        had_updates: bool,
        result: &Result<(), StatsigErr>,
    ) {
        // Threshold 0 is the explicit off switch (`Some(0)` in StatsigOptions).
        if trigger != SpecsSyncTrigger::Background
            || !self.allow_dcs_deltas
            || self.delta_no_progress_threshold == 0
        {
            return;
        }

        // A full (non-delta) sync means we're not in delta mode this round; a
        // successful one resets the streak so the count reflects *consecutive*
        // delta syncs (e.g. after a checksum-triggered fallback recovers).
        if !requested_deltas {
            if result.is_ok() {
                self.delta_no_progress_count.store(0, Ordering::SeqCst);
            }
            return;
        }

        // A network failure says nothing about the delta stream's progress; leave
        // the streak untouched rather than counting it as a stall.
        if matches!(result, Err(StatsigErr::NetworkError(_))) {
            return;
        }

        // The server reported no newer data (`has_updates: false`). The client is
        // already current, so there is nothing to recover — this is not a stall,
        // regardless of whether the (no-op) apply to the listener succeeded. A
        // forced full resync would just return `has_updates: false` again and
        // hit the same local failure, so idle periods must never accumulate
        // toward a forced resync. Reset the streak.
        if !had_updates {
            self.delta_no_progress_count.store(0, Ordering::SeqCst);
            return;
        }

        // Progress = a successful apply that advanced the LCUT. A successful apply
        // that carried updates but didn't advance, or a non-network apply error
        // (protobuf/json parse, lock failure, etc.), both mean the snapshot isn't
        // moving forward despite the server having newer data.
        let made_progress = result.is_ok() && lcut_advanced(pre_lcut, self.get_listener_lcut());
        if made_progress {
            self.delta_no_progress_count.store(0, Ordering::SeqCst);
            return;
        }

        let count = self.delta_no_progress_count.fetch_add(1, Ordering::SeqCst) + 1;
        let now_ms = Utc::now().timestamp_millis().max(0) as u64;
        if !forced_resync_due(
            count,
            self.delta_no_progress_threshold,
            self.last_forced_full_resync_at_ms.load(Ordering::SeqCst),
            now_ms,
            MIN_FORCED_FULL_RESYNC_INTERVAL_MS,
        ) {
            return;
        }

        self.use_deltas_next_request.store(false, Ordering::SeqCst);
        self.last_forced_full_resync_at_ms
            .store(now_ms, Ordering::SeqCst);
        self.delta_no_progress_count.store(0, Ordering::SeqCst);
        log_d!(
            TAG,
            "Delta specs sync made no LCUT progress across {} consecutive background syncs; forcing a full resync",
            count
        );
    }

    pub async fn run_background_sync(self: Arc<Self>) {
        let specs_info = self.read_current_specs_info();

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
        let mut deltas_used = self.use_deltas_next_request.load(Ordering::SeqCst);
        let response = self
            .fetch_specs_from_network(current_specs_info.clone(), trigger)
            .await;
        let (mut source_api, mut response_format, mut network_success) = match &response {
            Ok(response) => (
                response.loggable_api.clone(),
                get_specs_response_format(&response.data),
                NetworkSyncOutcome::Success,
            ),
            Err(_) => (
                get_api_from_url(&construct_specs_url(
                    self.specs_url.as_str(),
                    self.sdk_key.as_str(),
                )),
                SpecsResponseFormat::Unknown,
                NetworkSyncOutcome::Failure,
            ),
        };
        if let Ok(response) = &response {
            deltas_used = response.requested_deltas;
        }

        let ProcessSpecDataOutcome {
            mut result,
            mut had_updates,
        } = self.process_spec_data(response).await;

        if self.should_attempt_fallback(trigger, &result) {
            log_d!(TAG, "Falling back to statsig api");
            let fallback_args = self.get_request_args(&current_specs_info, trigger);
            deltas_used = fallback_args.deltas_enabled;
            let response = self.handle_fallback_request(fallback_args).await;
            match &response {
                Ok(response) => {
                    source_api = response.loggable_api.clone();
                    response_format = get_specs_response_format(&response.data);
                    network_success = NetworkSyncOutcome::Success;
                    deltas_used = response.requested_deltas;
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
            let outcome = self.process_spec_data(response).await;
            result = outcome.result;
            had_updates = outcome.had_updates;
        }

        self.maybe_force_full_resync_on_stalled_deltas(
            trigger,
            current_specs_info.lcut,
            deltas_used,
            had_updates,
            &result,
        );

        let process_success = !matches!(result.as_ref(), Err(StatsigErr::NetworkError(_)));
        log_config_sync_overall_latency(
            &self.ops_stats,
            sync_start_ms,
            source_api.as_str(),
            response_format.as_str(),
            network_success.as_bool(),
            process_success,
            result
                .as_ref()
                .err()
                .map_or_else(String::new, |e| e.to_string()),
            deltas_used,
        );

        result
    }

    async fn process_spec_data(
        &self,
        response: Result<NetworkResponse, NetworkError>,
    ) -> ProcessSpecDataOutcome {
        let mut resp = match response {
            Ok(resp) => resp,
            Err(e) => {
                return ProcessSpecDataOutcome {
                    result: Err(StatsigErr::NetworkError(e)),
                    had_updates: false,
                }
            }
        };
        let requested_deltas = resp.requested_deltas;

        // Detect a "no updates" response (`{"has_updates":false}`) before handing
        // the payload to the listener. Both the origin and the forward proxy
        // return this as small uncompressed JSON, never protobuf. We only peek
        // non-protobuf payloads: the JSON reader path rewinds the stream (so the
        // listener re-parse is unaffected), but the protobuf stream reader does
        // NOT rewind, so a JSON peek there would consume/corrupt the payload. A
        // protobuf response always carries real specs, so `had_updates = true`.
        let had_updates = if get_specs_response_format(&resp.data) == SpecsResponseFormat::Protobuf
        {
            true
        } else {
            !resp
                .data
                .deserialize_into::<SpecsResponseNoUpdates>()
                .map(|parsed| !parsed.has_updates)
                .unwrap_or(false)
        };

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

        ProcessSpecDataOutcome {
            result,
            had_updates,
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigHttpSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let specs_info = self.read_current_specs_info();
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

fn lcut_advanced(pre_lcut: Option<u64>, post_lcut: Option<u64>) -> bool {
    match (pre_lcut, post_lcut) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(pre), Some(post)) => post > pre,
    }
}

/// A forced full resync is due when the no-progress streak has reached the
/// threshold and enough wall-clock time has elapsed since the last forced resync
/// (or one has never been forced). The time gate re-arms on its own, so a stream
/// wedged at a single LCUT still recovers instead of being permanently disarmed.
///
/// If the wall clock moves backwards (NTP correction, VM suspend/resume) so that
/// `now_ms < last_forced_at_ms`, we treat the resync as due rather than
/// suppressing it until the clock catches up. Erring toward an extra forced
/// resync is safe (a resync of an already-current config returns a tiny
/// `has_updates: false` response), whereas suppressing could strand a wedged
/// client for an unbounded amount of time.
fn forced_resync_due(
    count: u32,
    threshold: u32,
    last_forced_at_ms: u64,
    now_ms: u64,
    min_interval_ms: u64,
) -> bool {
    if count < threshold {
        return false;
    }
    if last_forced_at_ms == 0 || now_ms < last_forced_at_ms {
        return true;
    }
    now_ms - last_forced_at_ms >= min_interval_ms
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
            .await
            .result;

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
            .await
            .result;

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
            .await
            .result;

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

    struct StalledLcutListener {
        lcut: u64,
    }

    impl SpecsUpdateListener for StalledLcutListener {
        fn did_receive_specs_update(&self, _update: SpecsUpdate) -> Result<(), StatsigErr> {
            Ok(())
        }

        fn get_current_specs_info(&self) -> SpecsInfo {
            SpecsInfo {
                lcut: Some(self.lcut),
                checksum: None,
                source: SpecsSource::Network,
                source_api: None,
            }
        }
    }

    fn accept_deltas_enabled(adapter: &StatsigHttpSpecsAdapter) -> bool {
        adapter
            .get_request_args(&SpecsInfo::empty(), SpecsSyncTrigger::Manual)
            .query_params
            .as_ref()
            .is_some_and(|p| p.contains_key("accept_deltas"))
    }

    fn delta_adapter() -> StatsigHttpSpecsAdapter {
        let options = StatsigOptions {
            enable_dcs_deltas: Some(true),
            ..StatsigOptions::default()
        };
        StatsigHttpSpecsAdapter::new(
            "secret-key",
            Some(&options),
            Some("https://example.com/v2/download_config_specs".to_string()),
        )
    }

    #[tokio::test]
    async fn test_force_full_resync_after_stalled_delta_syncs() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));
        assert!(accept_deltas_enabled(&adapter));

        // Stalled: delta carried updates (had_updates) but pre lcut == post lcut
        // (100) for THRESHOLD consecutive syncs.
        for _ in 0..DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }

        // The next request should be a full (non-delta) resync.
        assert!(!accept_deltas_enabled(&adapter));
    }

    #[tokio::test]
    async fn test_no_force_when_delta_syncs_progress() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        // Each sync advances the LCUT (pre 50 < post 100) -> counter resets.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 2) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(50),
                true,
                true,
                &Ok(()),
            );
        }

        assert!(accept_deltas_enabled(&adapter));
    }

    #[tokio::test]
    async fn test_does_not_repeat_full_resync_within_throttle_window() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        for _ in 0..DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }
        assert!(!accept_deltas_enabled(&adapter));

        // A successful full (non-delta) sync re-enables deltas.
        let reenable = adapter
            .process_spec_data(Ok(NetworkResponse {
                data: ResponseData::from_bytes(vec![]),
                loggable_api: "test-api".to_string(),
                requested_deltas: false,
            }))
            .await
            .result;
        assert!(reenable.is_ok());
        assert!(accept_deltas_enabled(&adapter));

        // Still stalling, but within the throttle window (the whole test runs in
        // well under MIN_FORCED_FULL_RESYNC_INTERVAL_MS), so no repeated force.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 2) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }
        assert!(accept_deltas_enabled(&adapter));
    }

    #[test]
    fn test_forced_resync_due_time_throttle() {
        let threshold = 10;
        let window = 300_000;
        // Below threshold: never due.
        assert!(!forced_resync_due(
            threshold - 1,
            threshold,
            0,
            1_000_000,
            window
        ));
        // At threshold, never forced before: due immediately.
        assert!(forced_resync_due(
            threshold, threshold, 0, 1_000_000, window
        ));
        // At threshold, but within the throttle window since the last force: not due.
        assert!(!forced_resync_due(
            threshold,
            threshold,
            1_000_000,
            1_000_000 + window - 1,
            window
        ));
        // At threshold, once the window has elapsed: due again (re-arms itself).
        assert!(forced_resync_due(
            threshold,
            threshold,
            1_000_000,
            1_000_000 + window,
            window
        ));
    }

    #[tokio::test]
    async fn test_counts_non_network_apply_failures_as_no_progress() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        // Delta payloads that keep failing to apply (non-network, non-checksum)
        // must still count toward the stall threshold.
        for _ in 0..DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Err(StatsigErr::LockFailure("apply failed".to_string())),
            );
        }

        assert!(!accept_deltas_enabled(&adapter));
    }

    #[tokio::test]
    async fn test_network_error_does_not_count_as_stall() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 3) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Err(StatsigErr::NetworkError(NetworkError::DisableNetworkOn(
                    "offline".to_string(),
                ))),
            );
        }

        assert_eq!(adapter.delta_no_progress_count.load(Ordering::SeqCst), 0);
        assert!(accept_deltas_enabled(&adapter));
    }

    #[tokio::test]
    async fn test_resets_count_on_successful_non_delta_sync() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        // Build up a partial (below-threshold) stall streak.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD - 1) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }
        assert_eq!(
            adapter.delta_no_progress_count.load(Ordering::SeqCst),
            DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD - 1
        );

        // A successful full (non-delta) sync clears the streak.
        adapter.maybe_force_full_resync_on_stalled_deltas(
            SpecsSyncTrigger::Background,
            Some(100),
            false,
            true,
            &Ok(()),
        );
        assert_eq!(adapter.delta_no_progress_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_no_update_response_not_counted_as_stall() {
        let adapter = delta_adapter();
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        // A `has_updates: false` response means the client is already current.
        // Even though the LCUT never advances, an idle config must never force a
        // resync.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 3) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                false, // had_updates = false (server reported no updates)
                &Ok(()),
            );
        }

        assert_eq!(adapter.delta_no_progress_count.load(Ordering::SeqCst), 0);
        assert!(accept_deltas_enabled(&adapter));

        // Even if the (no-op) apply to the listener fails, a `has_updates: false`
        // response is still idle, not a stall: a forced resync couldn't recover
        // it. The streak must stay at zero and deltas must remain enabled.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 3) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                false, // had_updates = false
                &Err(StatsigErr::LockFailure("boom".to_string())),
            );
        }

        assert_eq!(adapter.delta_no_progress_count.load(Ordering::SeqCst), 0);
        assert!(accept_deltas_enabled(&adapter));
    }

    #[test]
    fn test_forced_resync_due_backward_clock() {
        let threshold = 10;
        let window = 300_000;
        // Clock moved backwards since the last forced resync (now < last): treat
        // as due rather than suppressing recovery until the clock catches up.
        assert!(forced_resync_due(
            threshold, threshold, 1_000_000, 999_999, window
        ));
    }

    #[tokio::test]
    async fn test_configurable_no_progress_threshold() {
        let custom_threshold = 3;
        let options = StatsigOptions {
            enable_dcs_deltas: Some(true),
            dcs_delta_no_progress_threshold: Some(custom_threshold),
            ..StatsigOptions::default()
        };
        let adapter = StatsigHttpSpecsAdapter::new(
            "secret-key",
            Some(&options),
            Some("https://example.com/v2/download_config_specs".to_string()),
        );
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));

        // One short of the custom threshold: deltas still enabled.
        for _ in 0..(custom_threshold - 1) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }
        assert!(accept_deltas_enabled(&adapter));

        // Hitting the custom threshold forces a full resync.
        adapter.maybe_force_full_resync_on_stalled_deltas(
            SpecsSyncTrigger::Background,
            Some(100),
            true,
            true,
            &Ok(()),
        );
        assert!(!accept_deltas_enabled(&adapter));
    }

    #[tokio::test]
    async fn test_zero_threshold_disables_recovery() {
        let options = StatsigOptions {
            enable_dcs_deltas: Some(true),
            dcs_delta_no_progress_threshold: Some(0),
            ..StatsigOptions::default()
        };
        let adapter = StatsigHttpSpecsAdapter::new(
            "secret-key",
            Some(&options),
            Some("https://example.com/v2/download_config_specs".to_string()),
        );
        adapter.initialize(Arc::new(StalledLcutListener { lcut: 100 }));
        assert_eq!(adapter.delta_no_progress_threshold, 0);

        // Even a long stall streak must never force a resync when disabled.
        for _ in 0..(DEFAULT_DCS_DELTA_NO_PROGRESS_THRESHOLD * 3) {
            adapter.maybe_force_full_resync_on_stalled_deltas(
                SpecsSyncTrigger::Background,
                Some(100),
                true,
                true,
                &Ok(()),
            );
        }
        assert_eq!(adapter.delta_no_progress_count.load(Ordering::SeqCst), 0);
        assert!(accept_deltas_enabled(&adapter));
    }

    #[test]
    fn test_lcut_advanced_semantics() {
        assert!(lcut_advanced(Some(1), Some(2)));
        assert!(lcut_advanced(None, Some(1)));
        assert!(!lcut_advanced(Some(2), Some(2)));
        assert!(!lcut_advanced(Some(2), Some(1)));
        assert!(!lcut_advanced(Some(1), None));
    }

    #[test]
    fn test_get_response_format_json() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let data = ResponseData::from_bytes_with_headers(vec![], Some(headers));
        assert!(matches!(
            get_specs_response_format(&data),
            SpecsResponseFormat::Json
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
            get_specs_response_format(&data),
            SpecsResponseFormat::PlainText
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
            get_specs_response_format(&data),
            SpecsResponseFormat::Protobuf
        ));
    }

    #[test]
    fn test_get_response_format_unknown_without_content_type() {
        let data = ResponseData::from_bytes(vec![]);
        assert!(matches!(
            get_specs_response_format(&data),
            SpecsResponseFormat::Unknown
        ));
    }
}
