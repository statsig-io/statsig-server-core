use super::IdListMetadata;
use crate::id_lists_adapter::{IdListUpdate, IdListsAdapter, IdListsUpdateListener};
use crate::networking::{NetworkClient, NetworkError, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_error_to_statsig_and_console, log_w, StatsigErr, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
use serde_json::from_str;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

const DEFAULT_ID_LISTS_MANIFEST_URL: &str = "https://statsigapi.net/v1/get_id_lists";
const DEFAULT_ID_LIST_SYNC_INTERVAL_MS: u32 = 10_000;

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
    pub fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        let id_lists_manifest_url = options
            .id_lists_url
            .clone()
            .unwrap_or_else(|| DEFAULT_ID_LISTS_MANIFEST_URL.to_string());

        let fallback_url = if options.fallback_to_statsig_api.unwrap_or(false)
            && id_lists_manifest_url != DEFAULT_ID_LISTS_MANIFEST_URL
        {
            Some(DEFAULT_ID_LISTS_MANIFEST_URL.to_string())
        } else {
            None
        };

        let sync_interval_duration = Duration::from_millis(
            options
                .id_lists_sync_interval_ms
                .unwrap_or(DEFAULT_ID_LIST_SYNC_INTERVAL_MS) as u64,
        );

        let network = NetworkClient::new(
            sdk_key,
            Some(StatsigMetadata::get_constant_request_headers(sdk_key)),
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

    async fn fetch_id_list_manifests_from_network(&self) -> Result<IdListsResponse, StatsigErr> {
        let request_args = RequestArgs {
            url: self.id_lists_manifest_url.clone(),
            retries: 2,
            accept_gzip_response: true,
            ..RequestArgs::new()
        };

        let initial_err = match self.network.post(request_args.clone(), None).await {
            Ok(response) => return self.parse_response(response),
            Err(e) => e,
        };

        if initial_err != NetworkError::RetriesExhausted {
            return Err(StatsigErr::NetworkError(format!(
                "Initial request failed: {:?}",
                initial_err
            )));
        }

        // attempt fallback
        if let Some(fallback_url) = &self.fallback_url {
            let fallback_err = match self
                .handle_fallback_request(fallback_url, request_args)
                .await
            {
                Ok(response) => return self.parse_response(response),
                Err(e) => e,
            };

            // TODO logging
            return Err(StatsigErr::NetworkError(format!(
                "Fallback request failed: {:?}, initial error: {:?}",
                fallback_err, initial_err
            )));
        }

        Err(StatsigErr::NetworkError(format!(
            "Initial request failed with error: {:?}",
            initial_err
        )))
    }

    async fn fetch_individual_id_list_changes_from_network(
        &self,
        list_url: &str,
        list_size: u64,
    ) -> Result<String, StatsigErr> {
        let headers = HashMap::from([("Range".into(), format!("bytes={}-", list_size))]);

        let response = self
            .network
            .get(RequestArgs {
                url: list_url.to_string(),
                headers: Some(headers),
                ..RequestArgs::new()
            })
            .await;

        match response {
            Ok(data) => {
                if data.is_empty() {
                    let msg = "No ID List changes from network".to_string();
                    return Err(StatsigErr::NetworkError(msg));
                }
                Ok(data)
            }
            Err(err) => {
                let msg = format!("Failed to fetch ID List changes: {:?}", err);
                Err(StatsigErr::NetworkError(msg))
            }
        }
    }

    async fn handle_fallback_request(
        &self,
        fallback_url: &str,
        mut request_args: RequestArgs,
    ) -> Result<String, StatsigErr> {
        request_args.url = fallback_url.to_owned();

        // TODO add log

        match self.network.post(request_args.clone(), None).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let msg = format!("Fallback request failed: {:?}", e);
                Err(StatsigErr::NetworkError(msg))
            }
        }
    }
    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => return,
        };

        if let Err(e) = strong_self.sync_id_lists().await {
            log_w!(TAG, "IDList background sync failed {}", e);
        }
    }

    fn parse_response(&self, response: String) -> Result<IdListsResponse, StatsigErr> {
        if response.is_empty() {
            let msg = "No ID List results from network".to_string();
            return Err(StatsigErr::NetworkError(msg));
        }

        from_str::<IdListsResponse>(&response).map_err(|parse_err| {
            let msg = format!("Failed to parse JSON: {}", parse_err);
            StatsigErr::JsonParseError(stringify!(IdListsResponse).to_string(), msg)
        })
    }

    fn set_listener(&self, listener: Arc<dyn IdListsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),

            Err(e) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    "Failed to acquire write lock on listener: {}",
                    e
                );
            }
        }
    }

    fn get_current_id_list_metadata(&self) -> Result<HashMap<String, IdListMetadata>, StatsigErr> {
        let lock = match self.listener.read() {
            Ok(lock) => lock,
            Err(e) => return Err(StatsigErr::LockFailure(e.to_string())),
        };

        match lock.as_ref() {
            Some(listener) => Ok(listener.get_current_id_list_metadata()),
            None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
        }
    }

    async fn sync_id_lists(&self) -> Result<(), StatsigErr> {
        let manifest = self.fetch_id_list_manifests_from_network().await?;
        let metadata = self.get_current_id_list_metadata()?;

        let mut changes = HashMap::new();

        for (list_name, entry) in manifest {
            let (requires_download, range_start) = match metadata.get(&list_name) {
                Some(current) => (entry.size > current.size, current.size),
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

        match self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => {
                    listener.did_receive_id_list_updates(updates);
                    Ok(())
                }
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    "Failed to acquire read lock on listener: {}",
                    e
                );
                Err(StatsigErr::LockFailure(e.to_string()))
            }
        }
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
                        _ = sleep(interval_duration) => {
                            Self::run_background_sync(&weak_self).await;
                        }
                        _ = rt_shutdown_notify.notified() => {
                            log_d!(TAG, "Runtime shutdown. Shutting down id list background sync");
                            break;
                        },
                        _ = self.shutdown_notify.notified() => {
                            log_d!(TAG, "Shutting down id list background sync");
                            break;
                        }
                    }
                }
            },
        );
        Ok(())
    }

    fn get_type_name(&self) -> String {
        TAG.to_string()
    }
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
            let id_lists = self.id_lists.read().unwrap();
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
                .read()
                .unwrap()
                .iter()
                .map(|(key, list)| (key.clone(), list.metadata.clone()))
                .collect()
        }

        fn did_receive_id_list_updates(&self, updates: HashMap<String, IdListUpdate>) {
            let mut id_lists = self.id_lists.write().unwrap();

            // delete any id_lists that are not in the changesets
            id_lists.retain(|list_name, _| updates.contains_key(list_name));

            for (list_name, update) in updates {
                if let Some(entry) = id_lists.get_mut(&list_name) {
                    // update existing
                    entry.apply_update(&update);
                } else {
                    // add new
                    let mut list = IdList::new(update.new_metadata.clone());
                    list.apply_update(&update);
                    id_lists.insert(list_name.to_owned(), list);
                }
            }
        }
    }

    fn get_hashed_marcos() -> String {
        let hashed = HashUtil::new().sha256(&"Marcos".to_string());
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
            .replace("URL_REPLACE", &format!("{}/id_lists", mock_server_url));

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
            .replace("URL_REPLACE", &format!("{}/id_lists", mock_server_url));

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

        statsig_rt.shutdown_immediate();
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
