use crate::id_lists_adapter::id_lists_adapter::IdListsAdapter;
use crate::id_lists_adapter::{IdListEntry, IdListsResponse};
use crate::network_client::{NetworkClient, RequestArgs};
use crate::{log_e, StatsigErr, StatsigOptions};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde_json::from_str;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::{interval_at, Instant};

const DEFAULT_ID_LIST_URL: &str = "https://statsigapi.net/v1/get_id_lists";
const DEFAULT_ID_LIST_SYNC_INTERVAL_MS: u32 = 10_000;

pub struct StatsigHttpIdListsAdapter {
    id_lists_url: String,
    network: Arc<NetworkClient>,
    sync_interval_duration: Duration,
    shutdown_notify: Arc<Notify>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
    runtime_handle: RwLock<Option<Handle>>,
    id_lists: Arc<RwLock<HashMap<String, IdListEntry>>>,
}

impl StatsigHttpIdListsAdapter {
    pub fn new(_sdk_key: &str, options: &StatsigOptions, network_client: NetworkClient) -> Self {
        let id_lists_url = match &options.id_lists_url {
            Some(url) => url,
            None => DEFAULT_ID_LIST_URL,
        };

        Self {
            id_lists_url: id_lists_url.to_string(),
            network: Arc::new(network_client),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle: Mutex::new(None),
            runtime_handle: RwLock::new(None),
            sync_interval_duration: Duration::from_millis(
                options
                    .id_lists_sync_interval_ms
                    .unwrap_or(DEFAULT_ID_LIST_SYNC_INTERVAL_MS) as u64,
            ),
            id_lists: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        runtime_handle: &Handle,
    ) -> Result<(), StatsigErr> {
        let weak_self = Arc::downgrade(&self);

        let interval_duration = self.sync_interval_duration;
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        {
            let mut handle_lock = self
                .runtime_handle
                .write()
                .map_err(|_| StatsigErr::IdListsAdapterRuntimeHandleLockFailure)?;
            *handle_lock = Some(runtime_handle.clone());
        }

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
            Err(_) => Err(StatsigErr::BackgroundTaskLockFailure),
        }
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let strong_self = match weak_self.upgrade() {
            Some(strong_self) => strong_self,
            None => {
                log_e!("Background id lists sync failed. Adapter has been released.");
                return;
            }
        };

        if let Err(err) = strong_self.sync_id_lists().await {
            log_e!("Background id lists sync failed: {}", err);
        }
    }

    async fn fetch_id_lists_from_network(&self) -> Result<IdListsResponse, StatsigErr> {
        let res = self
            .network
            .post(RequestArgs {
                url: self.id_lists_url.clone(),
                retries: 2,
                ..RequestArgs::new()
            })
            .await;

        let response_data = match res {
            Some(r) => r,
            None => {
                log_e!("No id list result from network");
                return Err(StatsigErr::IdListsAdapterNetworkFailure);
            }
        };

        match from_str::<IdListsResponse>(&response_data) {
            Ok(id_lists) => Ok(id_lists),
            Err(e) => Err(StatsigErr::IdListsAdapterParsingFailure(e.to_string())),
        }
    }

    fn fetch_individual_id_list_entry_from_network(
        &self,
        url: String,
        mut entry: IdListEntry,
    ) -> Result<JoinHandle<Result<(), StatsigErr>>, StatsigErr> {
        let handle = self
            .runtime_handle
            .read()
            .ok()
            .and_then(|lock| lock.as_ref().cloned())
            .ok_or(StatsigErr::BackgroundTaskLockFailure)?;

        let list_size = entry.size;
        let weak_network = Arc::downgrade(&self.network);
        let weak_id_lists = Arc::downgrade(&self.id_lists);
        Ok(handle.spawn(async move {
            match (weak_network.upgrade(), weak_id_lists.upgrade()) {
                (Some(strong_network), Some(strong_id_lists)) => {
                    let headers = Some(HashMap::from([(
                        "Range".into(),
                        format!("bytes={}-", list_size),
                    )]));

                    let response = match strong_network
                        .get(RequestArgs {
                            url,
                            headers,
                            ..RequestArgs::new()
                        })
                        .await
                    {
                        Some(res) => res,
                        None => return Err(StatsigErr::IdListsAdapterNetworkFailure),
                    };

                    let lines: Vec<&str> = response.lines().collect();

                    for line in lines {
                        let trimmed = line.trim();
                        if trimmed.len() <= 1 {
                            continue;
                        }

                        let op = line.chars().next();
                        let id = &line[1..];

                        match op {
                            Some('+') => {
                                entry.loaded_ids.insert(id.to_string());
                            }
                            Some('-') => {
                                entry.loaded_ids.remove(id);
                            }
                            _ => continue,
                        }
                    }

                    Self::upsert_id_list_entry(strong_id_lists, &entry)
                }
                _ => Err(StatsigErr::CustomError("".to_string())),
            }
        }))
    }

    fn upsert_id_list_entry(
        id_lists: Arc<RwLock<HashMap<String, IdListEntry>>>,
        entry: &IdListEntry,
    ) -> Result<(), StatsigErr> {
        match id_lists.write() {
            Ok(mut lists) => {
                lists.insert(entry.name.clone(), entry.clone());
                Ok(())
            }
            Err(_e) => Err(StatsigErr::IdListsAdapterFailedToInsertIdList),
        }
    }
}

#[async_trait]
impl IdListsAdapter for StatsigHttpIdListsAdapter {
    async fn start(self: Arc<Self>, runtime_handle: &Handle) -> Result<(), StatsigErr> {
        self.schedule_background_sync(runtime_handle)?;
        Ok(())
    }

    async fn sync_id_lists(&self) -> Result<(), StatsigErr> {
        let response = self.fetch_id_lists_from_network().await?;

        let mut jobs = FuturesUnordered::new();

        for (_, new_entry) in response {
            let existing_entry = {
                self.id_lists
                    .read()
                    .ok()
                    .and_then(|lists| lists.get(&new_entry.name).cloned())
                    .unwrap_or_else(|| {
                        let mut cloned = new_entry.clone();
                        cloned.size = 0;
                        cloned
                    })
            };

            Self::upsert_id_list_entry(self.id_lists.clone(), &existing_entry)?;

            if new_entry.creation_time < existing_entry.creation_time {
                continue;
            }

            let (new_url, new_file_id) = match (&new_entry.url, &new_entry.file_id) {
                (Some(u), Some(i)) => (u, i),
                _ => continue,
            };

            if Some(new_file_id) != existing_entry.file_id.as_ref() {
                let mut cloned = new_entry.clone();
                cloned.size = 0;
            }

            if new_entry.size <= existing_entry.size {
                continue;
            }

            match self.fetch_individual_id_list_entry_from_network(new_url.clone(), new_entry) {
                Ok(handle) => jobs.push(handle),
                Err(e) => log_e!("Failed to sync individual ID List {}", e),
            };
        }

        while let Some(result) = jobs.next().await {
            match result {
                Ok(r) => {
                    if let Err(e) = r {
                        log_e!("ID List task failed: {}", e);
                    }
                }
                Err(e) => {
                    log_e!("ID List task failed: {:?}", e);
                }
            }
        }

        Ok(())
    }

    fn does_list_contain_id(&self, list_name: &str, id: &str) -> bool {
        match self.id_lists.read() {
            Ok(id_lists) => match id_lists.get(list_name) {
                Some(list) => list.loaded_ids.contains(id),
                None => false,
            },
            Err(_) => {
                log_e!("Failed to get read lock for id list {}", list_name);
                false
            }
        }
    }

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let task_handle = {
            match self.task_handle.lock() {
                Ok(mut guard) => guard.take(),
                Err(_) => {
                    return Err(StatsigErr::CustomError(
                        "Failed to acquire lock to running task".to_string(),
                    ))
                }
            }
        };

        match task_handle {
            None => Err(StatsigErr::CustomError(
                "No running task to shut down".to_string(),
            )),
            Some(handle) => {
                let shutdown_future = handle;
                let shutdown_result = tokio::time::timeout(timeout, shutdown_future).await;

                if shutdown_result.is_err() {
                    return Err(StatsigErr::CustomError(
                        "Failed to gracefully shutdown StatsigSpecsAdapter.".to_string(),
                    ));
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::memo_sha_256::MemoSha256;

    use super::*;
    use mockito::Server;
    use std::fs;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_background_sync() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let id_lists_response_path = PathBuf::from(format!(
            "{}/tests/data/get_id_lists.json",
            env!("CARGO_MANIFEST_DIR")
        ));
        let id_lists_response = fs::read_to_string(id_lists_response_path)
            .unwrap()
            .replace("URL_REPLACE", &format!("{}/id_lists", url));

        let mock = server
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
            .replace("URL_REPLACE", &format!("{}/id_lists", url));

        let individual_list_mock = server
            .mock("GET", "/id_lists/company_id_list")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(company_ids_response)
            .create();

        let network = NetworkClient::new(None);
        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", url)),
            ..StatsigOptions::default()
        };

        let adapter = Arc::new(StatsigHttpIdListsAdapter::new("", &options, network));

        let handle = Handle::try_current().unwrap();
        adapter.clone().start(&handle).await.unwrap();
        adapter.sync_id_lists().await.unwrap();

        mock.assert();
        individual_list_mock.assert();

        let hashed = MemoSha256::new().hash_name(&"Marcos".to_string());
        let substring: String = hashed.chars().take(8).collect();

        let result = adapter.does_list_contain_id("company_id_list", &substring);
        assert!(result)
    }
}
