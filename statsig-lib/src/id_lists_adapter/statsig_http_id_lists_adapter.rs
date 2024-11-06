use crate::id_lists_adapter::{IdListUpdate, IdListsAdapter, IdListsUpdateListener};
use crate::network_client::{NetworkClient, RequestArgs};
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_e, log_w, StatsigErr, StatsigOptions};
use async_trait::async_trait;
use serde_json::from_str;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::{interval_at, Instant};

use super::IdListMetadata;

const DEFAULT_ID_LISTS_MANIFEST_URL: &str = "https://statsigapi.net/v1/get_id_lists";
const DEFAULT_ID_LIST_SYNC_INTERVAL_MS: u32 = 10_000;

type IdListsResponse = HashMap<String, IdListMetadata>;

pub struct StatsigHttpIdListsAdapter {
    id_lists_manifest_url: String,
    listener: RwLock<Option<Arc<dyn IdListsUpdateListener>>>,
    network: NetworkClient,
    sync_interval_duration: Duration,
    shutdown_notify: Arc<Notify>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

impl StatsigHttpIdListsAdapter {
    pub fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        Self {
            id_lists_manifest_url: options
                .id_lists_url
                .clone()
                .unwrap_or_else(|| DEFAULT_ID_LISTS_MANIFEST_URL.to_string()),
            listener: RwLock::new(None),
            network: NetworkClient::new(Some(StatsigMetadata::get_constant_request_headers(
                sdk_key,
            ))),
            shutdown_notify: Arc::new(Notify::new()),
            sync_interval_duration: Duration::from_millis(
                options
                    .id_lists_sync_interval_ms
                    .unwrap_or(DEFAULT_ID_LIST_SYNC_INTERVAL_MS) as u64,
            ),
            task_handle: Mutex::new(None),
        }
    }

    async fn fetch_id_list_manifests_from_network(&self) -> Result<IdListsResponse, StatsigErr> {
        let headers = HashMap::from([("Content-Length".into(), "0".to_string())]);

        let response = self.network.post(RequestArgs {
            url: self.id_lists_manifest_url.clone(),
            retries: 2,
            headers: Some(headers),
            ..RequestArgs::new()
        });

        let data = match response {
            Some(r) => r,
            None => {
                log_e!("No id list result from network");
                return Err(StatsigErr::NetworkError(
                    "No result from network".to_string(),
                ));
            }
        };

        match from_str::<IdListsResponse>(&data) {
            Ok(id_lists) => Ok(id_lists),
            Err(e) => Err(StatsigErr::JsonParseError(
                stringify!(IdListsResponse).to_string(),
                e.to_string(),
            )),
        }
    }

    async fn fetch_individual_id_list_changes_from_network(
        &self,
        list_url: &str,
        list_size: u64,
    ) -> Result<String, StatsigErr> {
        let headers = HashMap::from([("Range".into(), format!("bytes={}-", list_size))]);

        let response = self.network.get(RequestArgs {
            url: list_url.to_string(),
            headers: Some(headers),
            ..RequestArgs::new()
        });

        let data = match response {
            Some(r) => r,
            None => {
                log_e!("No id list result from network");
                return Err(StatsigErr::NetworkError(
                    "No result from network".to_string(),
                ));
            }
        };

        Ok(data)
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        runtime_handle: &Handle,
    ) -> Result<(), StatsigErr> {
        let weak_self = Arc::downgrade(&self);

        let interval_duration = self.sync_interval_duration;
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        let handle = runtime_handle.spawn(async move {
            let mut interval = interval_at(Instant::now() + interval_duration, interval_duration);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::run_background_sync(&weak_self).await
                    }
                    _ = shutdown_notify.notified() => {
                        break;
                    }
                }
            }
        });

        match self.task_handle.lock() {
            Ok(mut guard) => {
                *guard = Some(handle);
                Ok(())
            }
            Err(e) => Err(StatsigErr::LockFailure(e.to_string())),
        }
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(s) => s,
            None => return,
        };

        if let Err(e) = strong_self.sync_id_lists().await {
            log_w!("IDList background sync failed {}", e);
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
        runtime_handle: &Handle,
        listener: Arc<dyn IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
        }

        self.schedule_background_sync(runtime_handle)
    }

    async fn sync_id_lists(&self) -> Result<(), StatsigErr> {
        let manifest = self.fetch_id_list_manifests_from_network().await?;

        let mut changes = HashMap::new();

        if let Some(listener) = self.listener.read().unwrap().as_ref() {
            let metadata = listener.get_current_id_list_metadata();

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
        }

        let mut updates = HashMap::new();
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

        if let Some(listener) = self.listener.read().unwrap().as_ref() {
            listener.did_receive_id_list_updates(updates);
        }

        Ok(())
    }

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let task_handle = match self.task_handle.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => {
                return Err(StatsigErr::CustomError(
                    "Failed to acquire lock to running task".to_string(),
                ))
            }
        };

        match task_handle {
            None => Err(StatsigErr::CustomError(
                "No running task to shut down".to_string(),
            )),
            Some(handle) => {
                let shutdown_result = tokio::time::timeout(timeout, handle).await;

                if shutdown_result.is_err() {
                    return Err(StatsigErr::CustomError(
                        "Failed to gracefully shutdown StatsigHttpIdListsAdapter.".to_string(),
                    ));
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hashing::Hashing;
    use crate::id_lists_adapter::IdList;

    use super::*;
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

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    impl IdListsUpdateListener for TestIdListsUpdateListener {
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

        fn get_current_id_list_metadata(&self) -> HashMap<String, IdListMetadata> {
            self.id_lists
                .read()
                .unwrap()
                .iter()
                .map(|(key, list)| (key.clone(), list.metadata.clone()))
                .collect()
        }
    }

    fn get_hashed_marcos() -> String {
        let hashed = Hashing::new().sha256(&"Marcos".to_string());
        return hashed.chars().take(8).collect();
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

        let handle = Handle::try_current().unwrap();
        adapter
            .clone()
            .start(&handle, listener.clone())
            .await
            .unwrap();

        (server, adapter, listener)
    }

    #[tokio::test]
    async fn test_syncing_new_id_lists() {
        let (_server, adapter, listener) = setup(None).await;

        adapter.sync_id_lists().await.unwrap();

        let result = listener.does_list_contain_id("company_id_list", &get_hashed_marcos());
        assert!(result);
    }

    #[tokio::test]
    async fn test_syncing_deleting_id_lists() {
        let (mut server, adapter, listener) = setup(None).await;

        adapter.sync_id_lists().await.unwrap();

        server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create();

        adapter.sync_id_lists().await.unwrap();

        let result = listener.does_list_contain_id("company_id_list", &get_hashed_marcos());
        assert!(result == false);
    }

    #[tokio::test]
    async fn test_bg_syncing() {
        let (_server, _adapter, listener) = setup(Some(1)).await;

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
        let (_server, adapter, listener) = setup(Some(1)).await;

        let _ = adapter.shutdown(Duration::from_millis(1000)).await;

        let result = listener
            .does_list_contain_id_eventually(
                "company_id_list",
                &get_hashed_marcos(),
                Duration::from_millis(100),
            )
            .await;

        assert!(result == false);
    }

    // todo:
    // - test update id list
}
