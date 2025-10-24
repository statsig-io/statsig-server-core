use super::IdListMetadata;
use crate::id_lists_adapter::{IdListUpdate, IdListsAdapter, IdListsUpdateListener};
use crate::networking::{NetworkClient, NetworkError, RequestArgs, Response, ResponseData};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_error_to_statsig_and_console, log_w, StatsigErr, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
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
            Some(StatsigMetadata::get_constant_request_headers(sdk_key)),
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

        let initial_err = if self.is_cdn_url() {
            match self.network.get(request_args.clone()).await {
                Ok(response) => return self.parse_response(response.data),
                Err(e) => e,
            }
        } else {
            match self.network.post(request_args.clone(), None).await {
                Ok(response) => return self.parse_response(response.data),
                Err(e) => e,
            }
        };

        if !matches!(initial_err, NetworkError::RetriesExhausted(_, _, _, _)) {
            return Err(StatsigErr::NetworkError(initial_err));
        }

        // attempt fallback
        let mut fallback_err = None;
        if let Some(fallback_url) = &self.fallback_url {
            fallback_err = match self
                .handle_fallback_request(fallback_url, request_args)
                .await
            {
                Ok(response) => return self.parse_response(response.data),
                Err(e) => Some(e),
            };
        }

        Err(fallback_err.unwrap_or(StatsigErr::NetworkError(initial_err)))
    }

    async fn fetch_individual_id_list_changes_from_network(
        &self,
        list_url: &str,
        list_size: u64,
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
                ..RequestArgs::new()
            })
            .await
            .map_err(StatsigErr::NetworkError)?;

        let mut response_body = match response.data {
            Some(data) => data,
            None => {
                let msg = "No ID List changes from network".to_string();
                return Err(StatsigErr::JsonParseError("IdList".to_string(), msg));
            }
        };

        response_body.read_to_string().map_err(|err| {
            let msg = format!("Failed to parse ID List changes: {err:?}");
            StatsigErr::JsonParseError("IdList".to_string(), msg)
        })
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

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => return,
        };

        strong_self
            .ops_stats
            .set_diagnostics_context(ContextType::ConfigSync);

        if let Err(e) = strong_self.sync_id_lists().await {
            log_w!(TAG, "IDList background sync failed {}", e);
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
        let new_manifest = self.fetch_id_list_manifests_from_network().await?;
        let curr_manifest = self.get_current_id_list_metadata()?;

        let mut changes = HashMap::new();

        self.ops_stats.add_marker(
            Marker::new(
                KeyType::GetIDListSources,
                ActionType::Start,
                Some(StepType::Process),
            )
            .with_id_list_count(new_manifest.len()),
            None,
        );

        for (list_name, entry) in new_manifest {
            let (requires_download, range_start) = match curr_manifest.get(&list_name) {
                Some(current) => (
                    entry.size > current.size
                        || entry.creation_time > current.creation_time
                        || entry.file_id != current.file_id,
                    current.size,
                ),
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

            let data = self
                .fetch_individual_id_list_changes_from_network(
                    &new_metadata.url,
                    changeset.range_start,
                )
                .await?;

            updates.insert(
                list_name,
                IdListUpdate {
                    raw_changeset: Some(data),
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

        self.ops_stats.add_marker(
            Marker::new(
                KeyType::GetIDListSources,
                ActionType::End,
                Some(StepType::Process),
            )
            .with_is_success(result.is_ok()),
            None,
        );

        result
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
        self.sync_id_lists().await?;
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
