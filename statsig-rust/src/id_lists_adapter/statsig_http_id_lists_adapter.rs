use super::IdListMetadata;
use crate::id_lists_adapter::{IdListUpdate, IdListsAdapter, IdListsUpdateListener};
use crate::networking::{NetworkClient, NetworkError, RequestArgs, Response, ResponseData};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::statsig_metadata::StatsigMetadata;
use crate::utils::split_host_and_path;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, log_w, StatsigErr, StatsigOptions,
    StatsigRuntime,
};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

const STATSIG_CDN_URL: &str = "https://api.statsigcdn.com";
const DEFAULT_CDN_ID_LISTS_MANIFEST_URL: &str = "https://api.statsigcdn.com/v1/get_id_lists";
const DEFAULT_ID_LIST_SYNC_INTERVAL_MS: u32 = 60_000;

type IdListsResponse = HashMap<String, IdListMetadata>;

const TAG: &str = stringify!(StatsigHttpIdListsAdapter);
const ID_LISTS_SYNC_OVERALL_LATENCY_METRIC: &str = "id_lists_sync_overall.latency";
const ID_LISTS_SYNC_OVERALL_MANIFEST_SUCCESS_TAG: &str = "id_list_manifest_success";
const ID_LISTS_SYNC_OVERALL_SUCCEED_SINGLE_ID_LIST_NUMBER_TAG: &str =
    "succeed_single_id_list_number";
const ID_LISTS_SYNC_OVERALL_FAILED_SINGLE_ID_LIST_NUMBER_TAG: &str = "failed_single_id_list_number";

pub struct StatsigHttpIdListsAdapter {
    id_lists_manifest_url: String,
    download_id_list_file_api: Option<String>,
    fallback_url: Option<String>,
    listener: RwLock<Option<Arc<dyn IdListsUpdateListener>>>,
    network: NetworkClient,
    sync_interval_duration: Duration,
    request_timeout_ms: u64,
    ops_stats: Arc<OpsStatsForInstance>,
    shutdown_notify: Arc<Notify>,
    shutdown_generation: AtomicU64,
}

impl StatsigHttpIdListsAdapter {
    #[must_use]
    pub fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        let id_lists_manifest_url = match &options.id_lists_url {
            Some(url) => url.clone(),
            None => make_default_cdn_url(sdk_key),
        };

        let mut fallback_url = None;
        if options.fallback_to_statsig_api == Some(true)
            && !id_lists_manifest_url.contains(DEFAULT_CDN_ID_LISTS_MANIFEST_URL)
        {
            fallback_url = Some(make_default_cdn_url(sdk_key));
        }

        let sync_interval_duration = Duration::from_millis(u64::from(
            options
                .id_lists_sync_interval_ms
                .unwrap_or(DEFAULT_ID_LIST_SYNC_INTERVAL_MS),
        ));

        let network = NetworkClient::new(
            sdk_key,
            Some(StatsigMetadata::get_constant_request_headers(
                sdk_key,
                options.service_name.as_deref(),
            )),
            Some(options),
        );

        Self {
            id_lists_manifest_url,
            download_id_list_file_api: options.download_id_list_file_api.clone(),
            fallback_url,
            listener: RwLock::new(None),
            network,
            sync_interval_duration,
            // 0 means "use the network provider's default timeout".
            request_timeout_ms: options.id_lists_request_timeout_ms.unwrap_or(0),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            shutdown_notify: Arc::new(Notify::new()),
            shutdown_generation: AtomicU64::new(0),
        }
    }

    pub fn force_shutdown(&self) {
        self.signal_shutdown();
    }

    fn signal_shutdown(&self) {
        self.shutdown_generation.fetch_add(1, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        self.shutdown_notify.notify_one();
    }

    fn build_manifest_request_args(&self) -> RequestArgs {
        RequestArgs {
            url: self.id_lists_manifest_url.clone(),
            accept_gzip_response: true,
            diagnostics_key: Some(KeyType::GetIDListSources),
            timeout_ms: self.request_timeout_ms,
            ..RequestArgs::new()
        }
    }

    async fn fetch_id_list_manifests_from_network(&self) -> Result<IdListsResponse, StatsigErr> {
        let request_args = self.build_manifest_request_args();

        let result = if self.is_cdn_url() {
            self.network.get(request_args.clone()).await
        } else {
            self.network.post(request_args.clone(), None).await
        };

        let result = match result {
            Ok(response) => self.parse_response(response.data),
            Err(initial_err) => {
                self.handle_manifest_network_error(request_args, initial_err)
                    .await
            }
        };

        let (metric_name, metric_value) = if result.is_ok() {
            ("id_list_manifest_download_success", 1.0)
        } else {
            ("id_list_manifest_download_failure", 1.0)
        };
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Increment,
            metric_name.to_string(),
            metric_value,
            None,
        ));

        result
    }

    fn build_id_list_download_request_args(
        &self,
        list_url: &str,
        start_index: u64,
        new_file_size: u64,
        id_list_file_id: Option<String>,
    ) -> RequestArgs {
        let list_url = self.get_override_id_list_download_url(list_url);
        // Full downloads (start 0) from the CDN send the range as a header, so Cloudflare caches
        // the file and serves it from cache (STADXP-549). The cached copy can be up to the edge
        // TTL old; the next sync picks up the missing tail below. Incremental downloads keep the
        // `range` query param, which bypasses the cache and goes to origin, so appended IDs stay
        // fresh and a stale cached copy can never answer them with a 416.
        let (headers, query_params) = if list_url.starts_with(STATSIG_CDN_URL) && start_index > 0 {
            (
                None,
                Some(HashMap::from([("range".into(), format!("{start_index}-"))])),
            )
        } else {
            let mut headers = HashMap::from([("Range".into(), format!("bytes={start_index}-"))]);
            headers.insert(
                "statsig-id-list-file-size".into(),
                new_file_size.to_string(),
            );
            (Some(headers), None)
        };

        RequestArgs {
            url: list_url,
            headers,
            query_params,
            id_list_file_id,
            diagnostics_key: Some(KeyType::GetIDList),
            timeout_ms: self.request_timeout_ms,
            ..RequestArgs::new()
        }
    }

    async fn fetch_individual_id_list_changes_from_network(
        &self,
        list_url: &str,
        start_index: u64,
        new_file_size: u64,
        id_list_file_id: Option<String>,
    ) -> Result<String, StatsigErr> {
        let request_args = self.build_id_list_download_request_args(
            list_url,
            start_index,
            new_file_size,
            id_list_file_id,
        );
        // Resolved (post-override) URL, reused below for diagnostics markers.
        let list_url = request_args.url.clone();

        let response = self.network.get(request_args).await;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                return Err(StatsigErr::NetworkError(err));
            }
        };

        self.add_diagnostics_start_marker(
            KeyType::GetIDList,
            StepType::Process,
            None,
            Some(list_url.clone()),
        );

        let mut response_body = match response.data {
            Some(data) => data,
            None => {
                let msg = "No ID List changes from network".to_string();
                self.add_diagnostics_end_marker(
                    KeyType::GetIDList,
                    StepType::Process,
                    false,
                    None,
                    Some(list_url.clone()),
                );
                return Err(StatsigErr::JsonParseError("IdList".to_string(), msg));
            }
        };

        let result = response_body
            .read_to_string()
            .map_err(|err| {
                let msg = format!("Failed to parse ID List changes: {err:?}");
                StatsigErr::JsonParseError("IdList".to_string(), msg)
            })
            .and_then(validate_id_list_changes);

        self.add_diagnostics_end_marker(
            KeyType::GetIDList,
            StepType::Process,
            result.is_ok(),
            None,
            Some(list_url),
        );

        result
    }

    fn get_override_id_list_download_url(&self, list_url: &str) -> String {
        let Some(override_api) = &self.download_id_list_file_api else {
            return list_url.to_string();
        };

        let (_, path) = split_host_and_path(list_url);
        if path.is_empty() {
            return override_api.to_string();
        }

        format!("{override_api}/{path}")
    }

    async fn handle_fallback_request(
        &self,
        fallback_url: &str,
        mut request_args: RequestArgs,
    ) -> Result<Response, StatsigErr> {
        request_args.url = fallback_url.to_owned();

        // TODO add log

        // fallback to cdn, it's a get request
        match self.network.get(request_args.clone()).await {
            Ok(response) => Ok(response),
            Err(e) => Err(StatsigErr::NetworkError(e)),
        }
    }

    async fn handle_manifest_network_error(
        &self,
        request_args: RequestArgs,
        initial_err: NetworkError,
    ) -> Result<IdListsResponse, StatsigErr> {
        if !matches!(initial_err, NetworkError::RetriesExhausted(_, _, _, _)) {
            return Err(StatsigErr::NetworkError(initial_err));
        }

        if let Some(fallback_url) = &self.fallback_url {
            return match self
                .handle_fallback_request(fallback_url, request_args)
                .await
            {
                Ok(response) => self.parse_response(response.data),
                Err(e) => Err(e),
            };
        }

        Err(StatsigErr::NetworkError(initial_err))
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => return,
        };

        strong_self
            .ops_stats
            .set_diagnostics_context(ContextType::ConfigSync);

        if let Err(e) = strong_self.sync_id_lists(false).await {
            if let StatsigErr::NetworkError(NetworkError::DisableNetworkOn(_)) = e {
                return;
            }
            log_e!(TAG, "IDList background sync failed {}", e);
        }

        strong_self.ops_stats.enqueue_diagnostics_event(
            Some(KeyType::GetIDListSources),
            Some(ContextType::ConfigSync),
        );
    }

    fn is_cdn_url(&self) -> bool {
        self.id_lists_manifest_url
            .starts_with(DEFAULT_CDN_ID_LISTS_MANIFEST_URL)
    }

    fn parse_response(
        &self,
        response: Option<ResponseData>,
    ) -> Result<IdListsResponse, StatsigErr> {
        let mut data = match response {
            Some(r) => r,
            None => {
                let msg = "No ID List results from network".to_string();
                return Err(StatsigErr::JsonParseError(
                    "IdListsResponse".to_owned(),
                    msg,
                ));
            }
        };

        data.deserialize_into::<IdListsResponse>()
            .map_err(|parse_err| {
                let msg = format!("Failed to parse JSON: {parse_err}");
                StatsigErr::JsonParseError(stringify!(IdListsResponse).to_string(), msg)
            })
    }

    fn set_listener(&self, listener: Arc<dyn IdListsUpdateListener>) {
        match self
            .listener
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut lock) => *lock = Some(listener),
            None => {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    StatsigErr::LockFailure("Failed to acquire write lock on listener".to_string())
                );
            }
        }
    }

    fn get_current_id_list_metadata(&self) -> Result<HashMap<String, IdListMetadata>, StatsigErr> {
        let lock = match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => lock,
            None => {
                return Err(StatsigErr::LockFailure(
                    "Failed to acquire read lock on listener".to_string(),
                ))
            }
        };

        match lock.as_ref() {
            Some(listener) => Ok(listener.get_current_id_list_metadata()),
            None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
        }
    }

    // `fail_on_any_download_error`: init passes true so `initialize()` reports a missing list;
    // background sync passes false so one stuck list doesn't fail the whole sync. Either way,
    // the lists that downloaded are applied.
    async fn sync_id_lists(&self, fail_on_any_download_error: bool) -> Result<(), StatsigErr> {
        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        let mut id_list_manifest_success = false;
        let mut successful_single_id_list_number = 0_u64;
        let mut failed_single_id_list_number = 0_u64;

        let new_manifest = match self.fetch_id_list_manifests_from_network().await {
            Ok(manifest) => {
                id_list_manifest_success = true;
                manifest
            }
            Err(e) => {
                self.log_id_lists_sync_overall_latency(
                    sync_start_ms,
                    id_list_manifest_success,
                    successful_single_id_list_number,
                    failed_single_id_list_number,
                );
                return Err(e);
            }
        };

        let curr_manifest = match self.get_current_id_list_metadata() {
            Ok(manifest) => manifest,
            Err(e) => {
                self.log_id_lists_sync_overall_latency(
                    sync_start_ms,
                    id_list_manifest_success,
                    successful_single_id_list_number,
                    failed_single_id_list_number,
                );
                return Err(e);
            }
        };

        let mut changes = HashMap::new();

        self.add_diagnostics_start_marker(
            KeyType::GetIDListSources,
            StepType::Process,
            Some(new_manifest.len()),
            None,
        );

        for (list_name, entry) in new_manifest {
            let (requires_download, range_start) = match curr_manifest.get(&list_name) {
                Some(current) => {
                    if entry.creation_time > current.creation_time
                        || entry.file_id != current.file_id
                    {
                        // An empty file has nothing to download. The update still goes out
                        // without data, so a rotation to an empty file resets the list.
                        (entry.size > 0, 0u64)
                    } else if entry.size > current.size {
                        (true, current.size)
                    } else {
                        (false, 0u64)
                    }
                }
                None => (entry.size > 0, 0), // size 0: add an empty list, like legacy
            };

            changes.insert(
                list_name.clone(),
                IdListChangeSet {
                    new_metadata: entry,
                    requires_download,
                    range_start,
                },
            );
        }

        let mut updates = HashMap::new();
        let mut first_download_error: Option<StatsigErr> = None;
        // todo: map this into futures then run them in parallel
        for (list_name, changeset) in changes {
            let new_metadata = changeset.new_metadata;

            if !changeset.requires_download {
                updates.insert(
                    list_name,
                    IdListUpdate {
                        raw_changeset: None,
                        new_metadata,
                    },
                );
                continue;
            }

            let single_id_list_download_result = self
                .fetch_individual_id_list_changes_from_network(
                    &new_metadata.url,
                    changeset.range_start,
                    new_metadata.size,
                    new_metadata.file_id.clone(),
                )
                .await;

            let raw_changeset = match single_id_list_download_result {
                Ok(raw_changeset) => raw_changeset,
                Err(e) => {
                    // One failed list must not block the others. Keep this list as it is and
                    // retry next sync. The listener deletes lists missing from `updates`, so an
                    // existing list is re-sent with its current metadata (no reset, no size
                    // change); a new list is left out, as it doesn't exist locally yet.
                    log_w!(TAG, "Failed to download ID list {}: {}", list_name, e);
                    if let Some(current) = curr_manifest.get(&list_name) {
                        updates.insert(
                            list_name,
                            IdListUpdate {
                                raw_changeset: None,
                                new_metadata: current.clone(),
                            },
                        );
                    }
                    failed_single_id_list_number += 1;
                    first_download_error.get_or_insert(e);
                    continue;
                }
            };
            successful_single_id_list_number += 1;

            updates.insert(
                list_name,
                IdListUpdate {
                    raw_changeset: Some(raw_changeset),
                    new_metadata,
                },
            );
        }

        let result = match self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => {
                    listener.did_receive_id_list_updates(updates);
                    Ok(())
                }
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            None => {
                let error = "Failed to acquire read lock on listener".to_string();
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    StatsigErr::LockFailure(error.clone())
                );
                Err(StatsigErr::LockFailure(error))
            }
        };

        // The lists that downloaded were applied above. Surface a download error on init, or when
        // every download failed, so total outages still show up as a failed sync.
        let all_failed = successful_single_id_list_number == 0;
        let result = match (result, first_download_error) {
            (Ok(()), Some(err)) if fail_on_any_download_error || all_failed => Err(err),
            (result, _) => result,
        };

        self.add_diagnostics_end_marker(
            KeyType::GetIDListSources,
            StepType::Process,
            result.is_ok(),
            None,
            None,
        );

        self.log_id_lists_sync_overall_latency(
            sync_start_ms,
            id_list_manifest_success,
            successful_single_id_list_number,
            failed_single_id_list_number,
        );

        result
    }

    // ---- Helper functions for monioring ID List ----
    fn add_diagnostics_start_marker(
        &self,
        key: KeyType,
        step: StepType,
        id_list_count: Option<usize>,
        url: Option<String>,
    ) {
        let mut marker = Marker::new(key, ActionType::Start, Some(step));
        if let Some(count) = id_list_count {
            marker = marker.with_id_list_count(count);
        }
        if let Some(url) = url {
            marker = marker.with_url(url);
        }
        self.ops_stats.add_marker(marker, None);
    }

    fn add_diagnostics_end_marker(
        &self,
        key: KeyType,
        step: StepType,
        success: bool,
        status_code: Option<u16>,
        url: Option<String>,
    ) {
        let mut marker = Marker::new(key, ActionType::End, Some(step)).with_is_success(success);
        if let Some(status_code) = status_code {
            marker = marker.with_status_code(status_code);
        }
        if let Some(url) = url {
            marker = marker.with_url(url);
        }
        self.ops_stats.add_marker(marker, None);
    }

    fn log_id_lists_sync_overall_latency(
        &self,
        sync_start_ms: u64,
        id_list_manifest_success: bool,
        successful_single_id_list_number: u64,
        failed_single_id_list_number: u64,
    ) {
        let latency_ms =
            (Utc::now().timestamp_millis() as u64).saturating_sub(sync_start_ms) as f64;
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            ID_LISTS_SYNC_OVERALL_LATENCY_METRIC.to_string(),
            latency_ms,
            Some(HashMap::from([
                (
                    ID_LISTS_SYNC_OVERALL_MANIFEST_SUCCESS_TAG.to_string(),
                    id_list_manifest_success.to_string(),
                ),
                (
                    ID_LISTS_SYNC_OVERALL_SUCCEED_SINGLE_ID_LIST_NUMBER_TAG.to_string(),
                    successful_single_id_list_number.to_string(),
                ),
                (
                    ID_LISTS_SYNC_OVERALL_FAILED_SINGLE_ID_LIST_NUMBER_TAG.to_string(),
                    failed_single_id_list_number.to_string(),
                ),
            ])),
        ));
    }
}

struct IdListChangeSet {
    new_metadata: IdListMetadata,
    requires_download: bool,
    range_start: u64,
}

#[async_trait]
impl IdListsAdapter for StatsigHttpIdListsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        self.set_listener(listener);
        self.ops_stats
            .set_diagnostics_context(ContextType::Initialize);
        let result = self.sync_id_lists(true).await;
        result?;
        Ok(())
    }

    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.signal_shutdown();
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let weak_self = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration;
        let shutdown_generation = self.shutdown_generation.load(Ordering::SeqCst);

        statsig_runtime.spawn(
            "http_id_list_bg_sync",
            move |rt_shutdown_notify| async move {
                loop {
                    tokio::select! {
                        () = sleep(interval_duration) => {
                            Self::run_background_sync(&weak_self).await;
                        }
                        () = rt_shutdown_notify.notified() => {
                            log_d!(TAG, "Runtime shutdown. Shutting down id list background sync");
                            break;
                        },
                        () = self.shutdown_notify.notified() => {
                            if self.shutdown_generation.load(Ordering::SeqCst)
                                != shutdown_generation
                            {
                                log_d!(TAG, "Shutting down id list background sync");
                                break;
                            }
                        }
                    }
                }
            },
        )?;

        Ok(())
    }

    fn get_type_name(&self) -> String {
        TAG.to_string()
    }
}

fn make_default_cdn_url(sdk_key: &str) -> String {
    format!("{DEFAULT_CDN_ID_LISTS_MANIFEST_URL}/{sdk_key}.json")
}

// Rejects bodies that aren't ID list changes (an error page, an empty body). The local size grows
// by the body length, so accepting one would throw every later range request off. Leading
// whitespace is allowed: a file without a trailing newline makes the next range start with '\n'.
fn validate_id_list_changes(body: String) -> Result<String, StatsigErr> {
    let first = body.trim_start().chars().next();
    if first != Some('+') && first != Some('-') {
        let msg = "ID List changes must start with '+' or '-'".to_string();
        return Err(StatsigErr::JsonParseError("IdList".to_string(), msg));
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use crate::hashing::HashUtil;
    use crate::id_lists_adapter::IdList;

    use super::*;
    // todo: update to use wiremock instead of mockito
    use mockito::{Mock, Server, ServerGuard};
    use std::fs;
    use std::path::PathBuf;

    struct TestIdListsUpdateListener {
        id_lists: RwLock<HashMap<String, IdList>>,
    }

    impl TestIdListsUpdateListener {
        fn does_list_contain_id(&self, list_name: &str, id: &str) -> bool {
            let id_lists = self
                .id_lists
                .try_read_for(std::time::Duration::from_secs(5))
                .unwrap();
            if let Some(list) = id_lists.get(list_name) {
                list.ids.contains(id)
            } else {
                false
            }
        }

        async fn does_list_contain_id_eventually(
            &self,
            list_name: &str,
            id: &str,
            timeout_duration: Duration,
        ) -> bool {
            let start = tokio::time::Instant::now();
            loop {
                if self.does_list_contain_id(list_name, id) {
                    return true;
                }

                if start.elapsed() >= timeout_duration {
                    return false;
                }

                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    impl IdListsUpdateListener for TestIdListsUpdateListener {
        fn get_current_id_list_metadata(&self) -> HashMap<String, IdListMetadata> {
            self.id_lists
                .try_read_for(std::time::Duration::from_secs(5))
                .unwrap()
                .iter()
                .map(|(key, list)| (key.clone(), list.metadata.clone()))
                .collect()
        }

        fn did_receive_id_list_updates(&self, updates: HashMap<String, IdListUpdate>) {
            let mut id_lists = self
                .id_lists
                .try_write_for(std::time::Duration::from_secs(5))
                .unwrap();

            // delete any id_lists that are not in the changesets
            id_lists.retain(|list_name, _| updates.contains_key(list_name));

            for (list_name, update) in updates {
                if let Some(entry) = id_lists.get_mut(&list_name) {
                    // update existing
                    entry.apply_update(update);
                } else {
                    // add new
                    let mut list = IdList::new(update.new_metadata.clone());
                    list.apply_update(update);
                    id_lists.insert(list_name.clone(), list);
                }
            }
        }
    }

    fn get_hashed_marcos() -> String {
        let hashed = HashUtil::new().sha256("Marcos");
        hashed.chars().take(8).collect()
    }

    async fn setup_mock_server() -> (ServerGuard, Mock, Mock) {
        let mut server = Server::new_async().await;
        let mock_server_url = server.url();

        let id_lists_response_path = PathBuf::from(format!(
            "{}/tests/data/get_id_lists.json",
            env!("CARGO_MANIFEST_DIR")
        ));

        let id_lists_response = fs::read_to_string(id_lists_response_path)
            .unwrap()
            .replace("URL_REPLACE", &format!("{mock_server_url}/id_lists"));

        let mocked_get_id_lists = server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(id_lists_response)
            .create();

        let company_ids_response_path = PathBuf::from(format!(
            "{}/tests/data/company_id_list",
            env!("CARGO_MANIFEST_DIR")
        ));

        let company_ids_response = fs::read_to_string(company_ids_response_path)
            .unwrap()
            .replace("URL_REPLACE", &format!("{mock_server_url}/id_lists"));

        let mocked_individual_id_list = server
            .mock("GET", "/id_lists/company_id_list")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(company_ids_response)
            .create();

        (server, mocked_get_id_lists, mocked_individual_id_list)
    }

    async fn setup(
        id_lists_sync_interval_ms: Option<u32>,
    ) -> (
        ServerGuard,
        Arc<StatsigHttpIdListsAdapter>,
        Arc<TestIdListsUpdateListener>,
        Arc<StatsigRuntime>,
    ) {
        let (server, _, _) = setup_mock_server().await;

        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", server.url())),
            id_lists_sync_interval_ms,
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
            ..StatsigOptions::default()
        };

        let adapter = Arc::new(StatsigHttpIdListsAdapter::new("secret-key", &options));
        let listener = Arc::new(TestIdListsUpdateListener {
            id_lists: RwLock::new(HashMap::new()),
        });

        let statsig_rt = StatsigRuntime::get_runtime();
        adapter
            .clone()
            .start(&statsig_rt, listener.clone())
            .await
            .unwrap();

        (server, adapter, listener, statsig_rt)
    }

    #[tokio::test]
    async fn test_id_list_download_uses_override_api() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;

        let id_lists_response_path = PathBuf::from(format!(
            "{}/tests/data/get_id_lists.json",
            env!("CARGO_MANIFEST_DIR")
        ));
        let id_lists_response = fs::read_to_string(id_lists_response_path).unwrap().replace(
            "URL_REPLACE",
            "https://fake-id-list-host/v1/download_id_list_file",
        );

        let mocked_get_id_lists = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(id_lists_response)
            .create();

        let company_ids_response_path = PathBuf::from(format!(
            "{}/tests/data/company_id_list",
            env!("CARGO_MANIFEST_DIR")
        ));
        let company_ids_response = fs::read_to_string(company_ids_response_path)
            .unwrap()
            .replace(
                "URL_REPLACE",
                "https://fake-id-list-host/v1/download_id_list_file",
            );

        let mocked_individual_id_list = download_server
            .mock("GET", "/v1/download_id_list_file/company_id_list")
            .match_header("range", "bytes=0-")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(company_ids_response)
            .create();

        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", manifest_server.url())),
            download_id_list_file_api: Some(download_server.url()),
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
            ..StatsigOptions::default()
        };

        let adapter = Arc::new(StatsigHttpIdListsAdapter::new("secret-key", &options));
        let listener = Arc::new(TestIdListsUpdateListener {
            id_lists: RwLock::new(HashMap::new()),
        });

        let statsig_rt = StatsigRuntime::get_runtime();
        adapter
            .clone()
            .start(&statsig_rt, listener.clone())
            .await
            .unwrap();

        mocked_get_id_lists.assert();
        mocked_individual_id_list.assert();
        assert!(listener.does_list_contain_id("company_id_list", &get_hashed_marcos()));
    }

    #[tokio::test]
    async fn test_id_list_download_passes_file_size_with_range_header() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;

        let existing_list_size = 10_u64;
        let new_list_size = existing_list_size + 1;
        let existing_creation_time = 1721417546000_i64;
        let file_id = "4t0BEqak3w1UcidsPcpQXN".to_string();
        let manifest_download_url = "https://fake-id-list-host/v1/download_id_list_file";
        let expected_range_start = existing_list_size;
        let expected_file_size = new_list_size.to_string();
        let expected_range_header = format!("bytes={expected_range_start}-");

        let manifest_response = format!(
            r#"{{
  "company_id_list": {{
    "name": "company_id_list",
    "size": {},
    "url": "{}/company_id_list",
    "creationTime": {},
    "fileID": "{}"
  }}
}}"#,
            new_list_size, manifest_download_url, existing_creation_time, file_id
        );

        let mocked_get_id_lists = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(manifest_response)
            .create();

        let mocked_individual_id_list = download_server
            .mock("GET", "/v1/download_id_list_file/company_id_list")
            .match_header("range", expected_range_header.as_str())
            .match_header("statsig-id-list-file-size", expected_file_size.as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("+2Tv4fIVX\n")
            .create();

        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", manifest_server.url())),
            download_id_list_file_api: Some(download_server.url()),
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
            ..StatsigOptions::default()
        };

        let adapter = Arc::new(StatsigHttpIdListsAdapter::new("secret-key", &options));
        let listener = Arc::new(TestIdListsUpdateListener {
            id_lists: RwLock::new(HashMap::new()),
        });

        {
            let mut existing_lists = listener
                .id_lists
                .try_write_for(std::time::Duration::from_secs(5))
                .unwrap();

            let mut local_list = IdList::new(IdListMetadata {
                name: "company_id_list".to_string(),
                url: format!("{}/company_id_list", manifest_download_url),
                file_id: Some(file_id),
                size: existing_list_size,
                creation_time: existing_creation_time,
            });
            local_list.metadata.size = existing_list_size;
            existing_lists.insert("company_id_list".to_string(), local_list);
        }

        let statsig_rt = StatsigRuntime::get_runtime();
        adapter
            .clone()
            .start(&statsig_rt, listener.clone())
            .await
            .unwrap();

        mocked_get_id_lists.assert();
        mocked_individual_id_list.assert();
        assert!(listener.does_list_contain_id("company_id_list", "2Tv4fIVX"));
    }

    #[test]
    fn test_override_id_list_download_url_preserves_manifest_path_suffix() {
        let options = StatsigOptions {
            download_id_list_file_api: Some("https://download-proxy.example".to_string()),
            ..StatsigOptions::default()
        };

        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &options);
        let manifest_download_url =
            "https://fake-id-list-host/v1/download_id_list_file/3wHgh0FhoQH0p";

        let actual = adapter.get_override_id_list_download_url(manifest_download_url);

        assert_eq!(
            actual,
            "https://download-proxy.example/v1/download_id_list_file/3wHgh0FhoQH0p"
        );
    }

    // Builds a download request against a non-CDN URL so we exercise the
    // header/range branch while asserting the timeout is applied.
    fn download_args(adapter: &StatsigHttpIdListsAdapter) -> RequestArgs {
        adapter.build_id_list_download_request_args(
            "https://fake-id-list-host/company_id_list",
            0,
            10,
            None,
        )
    }

    #[test]
    fn test_request_timeout_defaults_to_zero_when_unset() {
        // 0 is the "use the network provider's default timeout" sentinel, so an
        // unset option must not override the existing default behavior.
        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &StatsigOptions::default());
        assert_eq!(adapter.request_timeout_ms, 0);
        assert_eq!(adapter.build_manifest_request_args().timeout_ms, 0);
        assert_eq!(download_args(&adapter).timeout_ms, 0);
    }

    #[test]
    fn test_request_timeout_explicit_zero_uses_default() {
        // Some(0) is treated identically to unset: fall back to the default.
        let options = StatsigOptions {
            id_lists_request_timeout_ms: Some(0),
            ..StatsigOptions::default()
        };

        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &options);
        assert_eq!(adapter.request_timeout_ms, 0);
    }

    #[test]
    fn test_request_timeout_uses_configured_option() {
        let options = StatsigOptions {
            id_lists_request_timeout_ms: Some(25_000),
            ..StatsigOptions::default()
        };

        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &options);
        assert_eq!(adapter.request_timeout_ms, 25_000);

        // Both network request types (manifest fetch + per-file download) must
        // carry the configured timeout onto their RequestArgs.
        assert_eq!(adapter.build_manifest_request_args().timeout_ms, 25_000);
        assert_eq!(download_args(&adapter).timeout_ms, 25_000);
    }

    #[tokio::test]
    async fn test_syncing_new_id_lists() {
        let (_server, adapter, listener, _statsig_rt) = setup(None).await;

        adapter.sync_id_lists(false).await.unwrap();

        let result = listener.does_list_contain_id("company_id_list", &get_hashed_marcos());
        assert!(result);
    }

    #[tokio::test]
    async fn test_syncing_deleting_id_lists() {
        let (mut server, adapter, listener, _statsig_rt) = setup(None).await;

        adapter.sync_id_lists(false).await.unwrap();

        server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create();

        adapter.sync_id_lists(false).await.unwrap();

        let result = listener.does_list_contain_id("company_id_list", &get_hashed_marcos());
        assert!(!result);
    }

    #[tokio::test]
    async fn test_bg_syncing() {
        let (_server, _adapter, listener, _statsig_rt) = setup(Some(1)).await;

        let result = listener
            .does_list_contain_id_eventually(
                "company_id_list",
                &get_hashed_marcos(),
                Duration::from_millis(100),
            )
            .await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_bg_sync_shutdown() {
        let (_server, adapter, listener, statsig_rt) = setup(Some(10)).await;

        statsig_rt.shutdown();
        let _ = adapter.shutdown(Duration::from_millis(1)).await;

        let result = listener
            .does_list_contain_id_eventually(
                "company_id_list",
                &get_hashed_marcos(),
                Duration::from_millis(100),
            )
            .await;

        assert!(result);
    }

    // ---- STADXP-549: hybrid range requests and per-list failure isolation ----

    const CDN_LIST_URL: &str = "https://api.statsigcdn.com/v1/download_id_list_file/list_a";

    #[test]
    fn test_cdn_full_download_uses_range_header() {
        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &StatsigOptions::default());
        let args = adapter.build_id_list_download_request_args(CDN_LIST_URL, 0, 50_000, None);

        assert!(args.query_params.is_none());
        let headers = args.headers.expect("full CDN download sends headers");
        assert_eq!(headers.get("Range").map(String::as_str), Some("bytes=0-"));
        assert_eq!(
            headers.get("statsig-id-list-file-size").map(String::as_str),
            Some("50000")
        );
    }

    #[test]
    fn test_cdn_incremental_download_uses_range_query_param() {
        let adapter = StatsigHttpIdListsAdapter::new("secret-key", &StatsigOptions::default());
        let args = adapter.build_id_list_download_request_args(CDN_LIST_URL, 25_000, 50_000, None);

        assert!(args.headers.is_none());
        let query_params = args
            .query_params
            .expect("incremental CDN download sends ?range=");
        assert_eq!(
            query_params.get("range").map(String::as_str),
            Some("25000-")
        );
    }

    #[test]
    fn test_validate_rejects_body_that_is_not_id_list_changes() {
        assert!(validate_id_list_changes("<html>error</html>".into()).is_err());
        assert!(validate_id_list_changes(String::new()).is_err());
        assert!(validate_id_list_changes("\n".into()).is_err());
        assert!(validate_id_list_changes("+aaaaa\n-bbbbb\n".into()).is_ok());
        // Range starting right after a last line with no trailing newline.
        assert!(validate_id_list_changes("\n-aaaaa\n+zzzzz".into()).is_ok());
    }

    fn two_list_manifest(download_url: &str, list_a_size: u64) -> String {
        format!(
            r#"{{
  "list_a": {{"name": "list_a", "size": {list_a_size}, "url": "{download_url}/list_a", "creationTime": 1, "fileID": "file_a"}},
  "list_b": {{"name": "list_b", "size": 7, "url": "{download_url}/list_b", "creationTime": 1, "fileID": "file_b"}}
}}"#
        )
    }

    // list_a exists locally with "+aaaaa\n" (7 bytes); list_b is new.
    async fn start_with_existing_list_a(
        manifest: String,
        download_server: &ServerGuard,
        manifest_server: &mut ServerGuard,
    ) -> (Result<(), StatsigErr>, Arc<TestIdListsUpdateListener>) {
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(manifest)
            .create();

        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", manifest_server.url())),
            download_id_list_file_api: Some(download_server.url()),
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
            ..StatsigOptions::default()
        };
        let adapter = Arc::new(StatsigHttpIdListsAdapter::new("secret-key", &options));

        let mut list_a = IdList::new(IdListMetadata {
            name: "list_a".to_string(),
            url: format!("{}/v1/download_id_list_file/list_a", download_server.url()),
            file_id: Some("file_a".to_string()),
            size: 7,
            creation_time: 1,
        });
        list_a.apply_update(IdListUpdate {
            raw_changeset: Some("+aaaaa\n".to_string()),
            new_metadata: list_a.metadata.clone(),
        });
        let listener = Arc::new(TestIdListsUpdateListener {
            id_lists: RwLock::new(HashMap::from([("list_a".to_string(), list_a)])),
        });

        let result = adapter
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await;
        (result, listener)
    }

    #[tokio::test]
    async fn test_one_failed_download_keeps_that_list_and_applies_the_others() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let download_url = format!("{}/v1/download_id_list_file", download_server.url());

        // list_a grew, but its incremental download fails.
        let failed_a = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(416)
            .create();
        let ok_b = download_server
            .mock("GET", "/v1/download_id_list_file/list_b")
            .with_status(200)
            .with_body("+bbbbb\n")
            .create();

        let (result, listener) = start_with_existing_list_a(
            two_list_manifest(&download_url, 14),
            &download_server,
            &mut manifest_server,
        )
        .await;

        failed_a.assert();
        ok_b.assert();
        assert!(result.is_err(), "init reports the failed list");
        assert!(
            listener.does_list_contain_id("list_a", "aaaaa"),
            "list_a kept"
        );
        assert!(
            listener.does_list_contain_id("list_b", "bbbbb"),
            "list_b applied"
        );
        let metadata = listener.get_current_id_list_metadata();
        assert_eq!(
            metadata["list_a"].size, 7,
            "list_a size unchanged, so it retries"
        );
    }

    #[tokio::test]
    async fn test_invalid_body_is_treated_as_failed_download() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let download_url = format!("{}/v1/download_id_list_file", download_server.url());

        download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(206)
            .with_header("content-range", "bytes 7-13/14")
            .with_body("<html>")
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/list_b")
            .with_status(200)
            .with_body("+bbbbb\n")
            .create();

        let (result, listener) = start_with_existing_list_a(
            two_list_manifest(&download_url, 14),
            &download_server,
            &mut manifest_server,
        )
        .await;

        assert!(result.is_err(), "init reports the invalid body");
        assert_eq!(listener.get_current_id_list_metadata()["list_a"].size, 7);
        assert!(listener.does_list_contain_id("list_b", "bbbbb"));
    }

    #[tokio::test]
    async fn test_all_downloads_failing_returns_error_and_keeps_existing_lists() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let download_url = format!("{}/v1/download_id_list_file", download_server.url());

        download_server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .create();

        let (result, listener) = start_with_existing_list_a(
            two_list_manifest(&download_url, 14),
            &download_server,
            &mut manifest_server,
        )
        .await;

        assert!(result.is_err());
        assert!(
            listener.does_list_contain_id("list_a", "aaaaa"),
            "list_a kept"
        );
        assert!(
            !listener
                .get_current_id_list_metadata()
                .contains_key("list_b"),
            "new list that failed is not added"
        );
    }

    // ---- PR #3236 review: required tests ----

    fn adapter_for(
        manifest_server: &ServerGuard,
        download_server: &ServerGuard,
    ) -> Arc<StatsigHttpIdListsAdapter> {
        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", manifest_server.url())),
            download_id_list_file_api: Some(download_server.url()),
            ..StatsigOptions::default()
        };
        Arc::new(StatsigHttpIdListsAdapter::new("secret-key", &options))
    }

    fn empty_listener() -> Arc<TestIdListsUpdateListener> {
        Arc::new(TestIdListsUpdateListener {
            id_lists: RwLock::new(HashMap::new()),
        })
    }

    fn one_list_manifest(
        download_server: &ServerGuard,
        name: &str,
        size: u64,
        file_id: &str,
        creation_time: i64,
    ) -> String {
        format!(
            r#"{{"{name}": {{"name": "{name}", "size": {size}, "url": "{}/v1/download_id_list_file/{name}", "creationTime": {creation_time}, "fileID": "{file_id}"}}}}"#,
            download_server.url()
        )
    }

    // A full download served from a stale cache (7 of 21 bytes) is caught up by the next sync,
    // from exactly the local size, with no double count.
    #[tokio::test]
    async fn test_stale_full_download_is_caught_up_by_next_sync() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                21,
                "file_a",
                1,
            ))
            .create();
        let full = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .match_header("range", "bytes=0-")
            .with_status(200)
            .with_body("+aaaaa\n")
            .expect(1)
            .create();
        let tail = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .match_header("range", "bytes=7-")
            .with_status(206)
            .with_body("+bbbbb\n-aaaaa\n")
            .expect(1)
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();

        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();
        assert_eq!(listener.get_current_id_list_metadata()["list_a"].size, 7);

        adapter.sync_id_lists(false).await.unwrap();
        assert_eq!(listener.get_current_id_list_metadata()["list_a"].size, 21);
        assert!(listener.does_list_contain_id("list_a", "bbbbb"));
        assert!(!listener.does_list_contain_id("list_a", "aaaaa"));

        adapter.sync_id_lists(false).await.unwrap(); // caught up: no more downloads
        full.assert();
        tail.assert();
    }

    // A file with no trailing newline makes the next range start with '\n'. The '\n' is file
    // bytes, so it counts toward the local size.
    #[tokio::test]
    async fn test_incremental_after_missing_trailing_newline_keeps_size_in_step() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let first = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                6,
                "file_a",
                1,
            ))
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .match_header("range", "bytes=0-")
            .with_status(200)
            .with_body("+aaaaa")
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();
        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();

        first.remove();
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                20,
                "file_a",
                1,
            ))
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .match_header("range", "bytes=6-")
            .with_status(206)
            .with_body("\n-aaaaa\n+zzzzz")
            .create();
        adapter.sync_id_lists(false).await.unwrap();

        assert_eq!(listener.get_current_id_list_metadata()["list_a"].size, 20);
        assert!(!listener.does_list_contain_id("list_a", "aaaaa"));
        assert!(listener.does_list_contain_id("list_a", "zzzzz"));
    }

    #[test]
    fn test_validate_rejects_whitespace_only_and_padded_error_pages() {
        assert!(validate_id_list_changes("   \t\n".into()).is_err());
        assert!(validate_id_list_changes("\n<html>error</html>".into()).is_err());
        assert!(validate_id_list_changes("\r\n+aaaaa\n".into()).is_ok());
    }

    // A failed full download of a rotated file keeps the old file's IDs and metadata, and the
    // next sync retries the full download.
    #[tokio::test]
    async fn test_failed_rotation_keeps_old_list_and_retries_full_download() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let first = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                7,
                "file_a",
                1,
            ))
            .create();
        let old_file = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(200)
            .with_body("+aaaaa\n")
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();
        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();

        first.remove();
        old_file.remove();
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                7,
                "file_a2",
                2,
            ))
            .create();
        let failed = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(500)
            .create();
        let _ = adapter.sync_id_lists(false).await;

        let metadata = &listener.get_current_id_list_metadata()["list_a"];
        assert_eq!(metadata.file_id.as_deref(), Some("file_a"));
        assert!(listener.does_list_contain_id("list_a", "aaaaa"));

        failed.remove();
        let retried = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .match_header("range", "bytes=0-")
            .with_status(200)
            .with_body("+ccccc\n")
            .create();
        adapter.sync_id_lists(false).await.unwrap();
        retried.assert();
        assert_eq!(
            listener.get_current_id_list_metadata()["list_a"]
                .file_id
                .as_deref(),
            Some("file_a2")
        );
        assert!(listener.does_list_contain_id("list_a", "ccccc"));
        assert!(!listener.does_list_contain_id("list_a", "aaaaa"));
    }

    // Ask 2: a size-0 list is added without a download, like legacy.
    #[tokio::test]
    async fn test_size_zero_new_list_is_added_without_download() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "empty_list",
                0,
                "file_e",
                1,
            ))
            .create();
        let download = download_server
            .mock("GET", "/v1/download_id_list_file/empty_list")
            .expect(0)
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();

        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();

        download.assert();
        assert_eq!(
            listener.get_current_id_list_metadata()["empty_list"].size,
            0
        );
    }

    // Ask 1: init reports a failed list as not ready, but still applies the lists that worked.
    #[tokio::test]
    async fn test_init_reports_not_ready_when_any_download_fails() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let download_url = format!("{}/v1/download_id_list_file", download_server.url());
        download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(416)
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/list_b")
            .with_status(200)
            .with_body("+bbbbb\n")
            .create();

        let (init, listener) = start_with_existing_list_a(
            two_list_manifest(&download_url, 14),
            &download_server,
            &mut manifest_server,
        )
        .await;

        assert!(init.is_err(), "init reports the failed list");
        assert!(
            listener.does_list_contain_id("list_a", "aaaaa"),
            "list_a kept"
        );
        assert!(
            listener.does_list_contain_id("list_b", "bbbbb"),
            "list_b applied"
        );
    }

    // Ask 1: background sync keeps partial success.
    #[tokio::test]
    async fn test_background_sync_succeeds_when_some_downloads_fail() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let url = download_server.url();
        let manifest = |size: u64| {
            format!(
                r#"{{"a": {{"name": "a", "size": {size}, "url": "{url}/v1/download_id_list_file/a", "creationTime": 1, "fileID": "fa"}},
                "b": {{"name": "b", "size": {size}, "url": "{url}/v1/download_id_list_file/b", "creationTime": 1, "fileID": "fb"}}}}"#
            )
        };
        let first = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(manifest(7))
            .create();
        let a_full = download_server
            .mock("GET", "/v1/download_id_list_file/a")
            .with_status(200)
            .with_body("+aaaaa\n")
            .create();
        let b_full = download_server
            .mock("GET", "/v1/download_id_list_file/b")
            .with_status(200)
            .with_body("+bbbbb\n")
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();
        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();

        first.remove();
        a_full.remove();
        b_full.remove();
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(manifest(14))
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/a")
            .with_status(500)
            .create();
        download_server
            .mock("GET", "/v1/download_id_list_file/b")
            .with_status(206)
            .with_body("+ccccc\n")
            .create();

        assert!(adapter.sync_id_lists(false).await.is_ok());
        assert_eq!(
            listener.get_current_id_list_metadata()["a"].size,
            7,
            "a retries next sync"
        );
        assert!(listener.does_list_contain_id("b", "ccccc"));
    }

    // Ask 2, rotation case: a new fileID with size 0 resets the list without a download.
    #[tokio::test]
    async fn test_rotation_to_empty_file_resets_list_without_download() {
        let mut manifest_server = Server::new_async().await;
        let mut download_server = Server::new_async().await;
        let first = manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                7,
                "file_a",
                1,
            ))
            .create();
        let old_file = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .with_status(200)
            .with_body("+aaaaa\n")
            .create();
        let adapter = adapter_for(&manifest_server, &download_server);
        let listener = empty_listener();
        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();

        first.remove();
        old_file.remove();
        manifest_server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(one_list_manifest(
                &download_server,
                "list_a",
                0,
                "file_a2",
                2,
            ))
            .create();
        let download = download_server
            .mock("GET", "/v1/download_id_list_file/list_a")
            .expect(0)
            .create();
        adapter.sync_id_lists(false).await.unwrap();

        download.assert();
        let metadata = &listener.get_current_id_list_metadata()["list_a"];
        assert_eq!(metadata.file_id.as_deref(), Some("file_a2"));
        assert_eq!(metadata.size, 0);
        assert!(!listener.does_list_contain_id("list_a", "aaaaa"));
    }
}
