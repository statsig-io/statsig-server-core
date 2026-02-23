use super::IdListMetadata;
use crate::id_lists_adapter::{IdListUpdate, IdListsAdapter, IdListsUpdateListener};
use crate::networking::{NetworkClient, NetworkError, RequestArgs, Response, ResponseData};
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_e, log_error_to_statsig_and_console, StatsigErr, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
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

pub struct StatsigHttpIdListsAdapter {
    id_lists_manifest_url: String,
    fallback_url: Option<String>,
    listener: RwLock<Option<Arc<dyn IdListsUpdateListener>>>,
    network: NetworkClient,
    sync_interval_duration: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
    shutdown_notify: Arc<Notify>,
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
            fallback_url,
            listener: RwLock::new(None),
            network,
            sync_interval_duration,
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    pub fn force_shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    async fn fetch_id_list_manifests_from_network(&self) -> Result<IdListsResponse, StatsigErr> {
        let request_args = RequestArgs {
            url: self.id_lists_manifest_url.clone(),
            accept_gzip_response: true,
            diagnostics_key: Some(KeyType::GetIDListSources),
            ..RequestArgs::new()
        };

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

    async fn fetch_individual_id_list_changes_from_network(
        &self,
        list_url: &str,
        list_size: u64,
        id_list_file_id: Option<String>,
    ) -> Result<String, StatsigErr> {
        let (headers, query_params) = if list_url.starts_with(STATSIG_CDN_URL) {
            (
                None,
                Some(HashMap::from([("range".into(), format!("{list_size}-"))])),
            )
        } else {
            (
                Some(HashMap::from([(
                    "Range".into(),
                    format!("bytes={list_size}-"),
                )])),
                None,
            )
        };

        let response = self
            .network
            .get(RequestArgs {
                url: list_url.to_string(),
                headers,
                query_params,
                id_list_file_id,
                diagnostics_key: Some(KeyType::GetIDList),
                ..RequestArgs::new()
            })
            .await;

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
            Some(list_url.to_string()),
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
                    Some(list_url.to_string()),
                );
                return Err(StatsigErr::JsonParseError("IdList".to_string(), msg));
            }
        };

        let result = response_body.read_to_string().map_err(|err| {
            let msg = format!("Failed to parse ID List changes: {err:?}");
            StatsigErr::JsonParseError("IdList".to_string(), msg)
        });

        self.add_diagnostics_end_marker(
            KeyType::GetIDList,
            StepType::Process,
            result.is_ok(),
            None,
            Some(list_url.to_string()),
        );

        result
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

        if let Err(e) = strong_self.sync_id_lists().await {
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

    async fn sync_id_lists(&self) -> Result<(), StatsigErr> {
        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        let mut id_list_manifest_success = false;
        let mut successful_single_id_list_number = 0_u64;

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
                        (true, 0u64)
                    } else {
                        (entry.size > current.size, current.size)
                    }
                }
                None => (true, 0),
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
                    new_metadata.file_id.clone(),
                )
                .await;

            let raw_changeset = match single_id_list_download_result {
                Ok(raw_changeset) => raw_changeset,
                Err(e) => {
                    self.log_id_lists_sync_overall_latency(
                        sync_start_ms,
                        id_list_manifest_success,
                        successful_single_id_list_number,
                    );
                    return Err(e);
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
        let result = self.sync_id_lists().await;
        result?;
        Ok(())
    }

    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let weak_self = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration;

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
                            log_d!(TAG, "Shutting down id list background sync");
                            break;
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
    async fn test_syncing_new_id_lists() {
        let (_server, adapter, listener, _statsig_rt) = setup(None).await;

        adapter.sync_id_lists().await.unwrap();

        let result = listener.does_list_contain_id("company_id_list", &get_hashed_marcos());
        assert!(result);
    }

    #[tokio::test]
    async fn test_syncing_deleting_id_lists() {
        let (mut server, adapter, listener, _statsig_rt) = setup(None).await;

        adapter.sync_id_lists().await.unwrap();

        server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create();

        adapter.sync_id_lists().await.unwrap();

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

    // todo:
    // - test update id list
}
