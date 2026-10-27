use super::*;
use crate::data_store_interface::{DataStoreResponse, RequestPath};
use crate::data_store_lifecycle::SharedDataStore;
use crate::id_lists_adapter::{IdList, IdListMetadata, IdListUpdate};
use async_trait::async_trait;
use mockito::Server;
use parking_lot::Mutex;
use std::sync::atomic::AtomicUsize;
use tokio::sync::Notify;

struct SnapshotDataStore {
    values: Mutex<HashMap<String, String>>,
    failed_keys: Mutex<Vec<String>>,
    fail_child_read_number: AtomicUsize,
    child_reads: AtomicUsize,
    written_keys: Mutex<Vec<String>>,
    requested_keys: Mutex<Vec<String>>,
    polling_override: Option<bool>,
    supports_snapshot: bool,
    supports_snapshot_write: AtomicBool,
    set_attempts: AtomicUsize,
    block_next_get: AtomicBool,
    active_get: AtomicBool,
    get_started: Notify,
    block_next_set: AtomicBool,
    active_set: AtomicBool,
    set_started: Notify,
    shutdowns: AtomicUsize,
    fail_initialize: AtomicBool,
    writes: Mutex<Vec<(String, String)>>,
    fail_writes: AtomicBool,
}

impl SnapshotDataStore {
    fn new(values: HashMap<String, String>, supports_polling: bool) -> Self {
        Self {
            values: Mutex::new(values),
            failed_keys: Mutex::new(Vec::new()),
            fail_child_read_number: AtomicUsize::new(0),
            child_reads: AtomicUsize::new(0),
            written_keys: Mutex::new(Vec::new()),
            requested_keys: Mutex::new(Vec::new()),
            polling_override: Some(supports_polling),
            supports_snapshot: true,
            supports_snapshot_write: AtomicBool::new(false),
            set_attempts: AtomicUsize::new(0),
            block_next_get: AtomicBool::new(false),
            active_get: AtomicBool::new(false),
            get_started: Notify::new(),
            block_next_set: AtomicBool::new(false),
            active_set: AtomicBool::new(false),
            set_started: Notify::new(),
            shutdowns: AtomicUsize::new(0),
            fail_initialize: AtomicBool::new(false),
            writes: Mutex::new(Vec::new()),
            fail_writes: AtomicBool::new(false),
        }
    }

    fn with_snapshot_write_support(self) -> Self {
        self.supports_snapshot_write.store(true, Ordering::SeqCst);
        self
    }

    fn without_snapshot_support(mut self) -> Self {
        self.supports_snapshot = false;
        self
    }

    fn without_polling_override(mut self) -> Self {
        self.polling_override = None;
        self
    }

    fn with_failed_initialize(self) -> Self {
        self.fail_initialize.store(true, Ordering::SeqCst);
        self
    }

    fn set_values(&self, values: HashMap<String, String>) {
        *self.values.lock() = values;
    }
}

#[async_trait]
impl DataStoreTrait for SnapshotDataStore {
    async fn supports_feature(&self, feature: DataStoreFeature) -> bool {
        match feature {
            DataStoreFeature::ConfigSpecs => true,
            DataStoreFeature::IDListSnapshot => self.supports_snapshot,
            DataStoreFeature::IDListSnapshotWrite => {
                self.supports_snapshot_write.load(Ordering::SeqCst)
            }
        }
    }

    async fn support_polling_updates_for_feature(&self, feature: DataStoreFeature) -> Option<bool> {
        (feature == DataStoreFeature::IDListSnapshot)
            .then_some(self.polling_override)
            .flatten()
    }

    async fn initialize(&self) -> Result<(), StatsigErr> {
        if self.fail_initialize.load(Ordering::SeqCst) {
            return Err(StatsigErr::DataStoreFailure("initialization failed".into()));
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        assert!(!self.active_get.load(Ordering::SeqCst));
        assert!(!self.active_set.load(Ordering::SeqCst));
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn get(
        &self,
        key: &str,
    ) -> Result<crate::data_store_interface::DataStoreResponse, StatsigErr> {
        self.requested_keys.lock().push(key.to_string());
        if key.contains("|id_list::")
            && self.child_reads.fetch_add(1, Ordering::SeqCst) + 1
                == self.fail_child_read_number.load(Ordering::SeqCst)
        {
            return Err(StatsigErr::DataStoreFailure("child get failed".into()));
        }
        if self.failed_keys.lock().iter().any(|failed| failed == key) {
            return Err(StatsigErr::DataStoreFailure(format!(
                "get failed for {key}"
            )));
        }
        if self.block_next_get.swap(false, Ordering::SeqCst) {
            struct ActiveGet<'a>(&'a AtomicBool);
            impl Drop for ActiveGet<'_> {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::SeqCst);
                }
            }
            self.active_get.store(true, Ordering::SeqCst);
            let _active = ActiveGet(&self.active_get);
            self.get_started.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(DataStoreResponse {
            result: self.values.lock().get(key).cloned(),
            time: Some(1),
        })
    }

    async fn set(&self, key: &str, value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        self.set_attempts.fetch_add(1, Ordering::SeqCst);
        if self.block_next_set.swap(false, Ordering::SeqCst) {
            struct ActiveSet<'a>(&'a AtomicBool);
            impl Drop for ActiveSet<'_> {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::SeqCst);
                }
            }
            self.active_set.store(true, Ordering::SeqCst);
            let _active = ActiveSet(&self.active_set);
            self.set_started.notify_one();
            std::future::pending::<()>().await;
        }
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(StatsigErr::DataStoreFailure("write denied".to_string()));
        }
        self.values
            .lock()
            .insert(key.to_string(), value.to_string());
        self.writes
            .lock()
            .push((key.to_string(), value.to_string()));
        self.written_keys.lock().push(key.to_string());
        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        false
    }
}

struct TestIdListsListener {
    id_lists: RwLock<HashMap<String, IdList>>,
}

#[tokio::test]
async fn failed_store_initialization_recovers_on_scheduled_read() {
    let mut server = Server::new_async().await;
    let fallback = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    store.fail_initialize.store(true, Ordering::SeqCst);
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        Arc::new(SharedDataStore::new(store.clone())),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    assert!(store.requested_keys.lock().is_empty());
    store.fail_initialize.store(false, Ordering::SeqCst);
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    adapter.sync_scheduled_tick().await.unwrap();
    assert!(listener.contains("paid-users", "2Tv4fIVX"));
    fallback.assert_async().await;
}

impl TestIdListsListener {
    fn contains(&self, list_name: &str, id: &str) -> bool {
        self.id_lists
            .read()
            .get(list_name)
            .is_some_and(|list| list.ids.contains(id))
    }
}

impl IdListsUpdateListener for TestIdListsListener {
    fn get_current_id_list_metadata(&self) -> HashMap<String, IdListMetadata> {
        self.id_lists
            .read()
            .iter()
            .map(|(name, list)| (name.clone(), list.metadata.clone()))
            .collect()
    }

    fn get_current_id_lists_snapshot(&self) -> Result<HashMap<String, IdList>, StatsigErr> {
        Ok(self.id_lists.read().clone())
    }

    fn did_receive_id_list_updates(&self, updates: HashMap<String, IdListUpdate>) {
        let mut lists = self.id_lists.write();
        lists.retain(|name, _| updates.contains_key(name));
        for (name, update) in updates {
            if let Some(list) = lists.get_mut(&name) {
                list.apply_update(update);
            } else {
                let mut list = IdList::new(update.new_metadata.clone());
                list.apply_update(update);
                lists.insert(name, list);
            }
        }
    }

    fn replace_id_lists_snapshot(
        &self,
        snapshot: HashMap<String, IdList>,
    ) -> Result<(), StatsigErr> {
        *self.id_lists.write() = snapshot;
        Ok(())
    }
}

fn complete_snapshot_values(
    adapter: &StatsigDataStoreIdListsAdapter,
    list_name: &str,
    child: &str,
    remote_size: usize,
) -> HashMap<String, String> {
    HashMap::from([
        (
            adapter.manifest_key.clone(),
            format!(
                r#"{{"{list_name}":{{"name":"{list_name}","url":"","fileID":"file-1","size":{},"creationTime":1}}}}"#,
                remote_size
            ),
        ),
        (adapter.child_key(list_name), child.to_string()),
    ])
}

#[tokio::test]
async fn id_list_write_back_requires_both_read_and_write_capabilities() {
    for (read, write) in [(false, false), (false, true), (true, false), (true, true)] {
        for polling in [false, true] {
            let mut server = Server::new_async().await;
            let mut manifest = serde_json::json!({"remote": {
                "name": "remote", "url": format!("{}/remote", server.url()),
                "fileID": "file-1", "size": 10, "creationTime": 1
            }});
            let manifest_request = server
                .mock("POST", "/get_id_lists")
                .with_status(200)
                .with_body(manifest.to_string())
                .expect(1)
                .create_async()
                .await;
            let child_request = server
                .mock("GET", "/remote")
                .with_status(206)
                .with_body("+AAAAAAAA\n")
                .expect(1)
                .create_async()
                .await;
            let options = StatsigOptions {
                id_lists_url: Some(format!("{}/get_id_lists", server.url())),
                ..Default::default()
            };
            let mut store = SnapshotDataStore::new(HashMap::new(), polling);
            store.supports_snapshot = read;
            store.supports_snapshot_write.store(write, Ordering::SeqCst);
            let store = Arc::new(store);
            let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
                "secret-key",
                Arc::new(HashUtil::new()),
                &options,
                store.clone(),
            ));
            let listener = Arc::new(TestIdListsListener {
                id_lists: RwLock::new(HashMap::new()),
            });
            adapter
                .clone()
                .start(&StatsigRuntime::get_runtime(), listener.clone())
                .await
                .unwrap();
            assert!(listener.contains("remote", "AAAAAAAA"));
            let initial_attempts = store.set_attempts.load(Ordering::SeqCst);
            assert_eq!(
                initial_attempts,
                if read && write { 2 } else { 0 },
                "read={read}, write={write}"
            );
            manifest_request.assert_async().await;
            manifest_request.remove_async().await;
            manifest["remote"]["size"] = 20.into();
            let changed_manifest = server
                .mock("POST", "/get_id_lists")
                .with_status(200)
                .with_body(manifest.to_string())
                .expect(1)
                .create_async()
                .await;
            let changed_child = server
                .mock("GET", "/remote")
                .match_header("range", "bytes=10-")
                .with_status(206)
                .with_body("+BBBBBBBB\n")
                .expect(1)
                .create_async()
                .await;
            store.set_values(HashMap::new());
            adapter.sync_scheduled_tick().await.unwrap();
            assert!(listener.contains("remote", "AAAAAAAA"));
            assert!(listener.contains("remote", "BBBBBBBB"));
            assert_eq!(
                store.set_attempts.load(Ordering::SeqCst),
                if read && write { 4 } else { 0 }
            );
            if !read {
                assert!(store.requested_keys.lock().is_empty());
            }
            changed_manifest.assert_async().await;
            changed_child.assert_async().await;
            child_request.assert_async().await;
            if read && write {
                changed_manifest.remove_async().await;
                store.supports_snapshot_write.store(false, Ordering::SeqCst);
                manifest["remote"]["size"] = 30.into();
                let readonly_manifest = server
                    .mock("POST", "/get_id_lists")
                    .with_status(200)
                    .with_body(manifest.to_string())
                    .expect(1)
                    .create_async()
                    .await;
                let readonly_child = server
                    .mock("GET", "/remote")
                    .match_header("range", "bytes=20-")
                    .with_status(206)
                    .with_body("+CCCCCCCC\n")
                    .expect(1)
                    .create_async()
                    .await;
                store.set_values(HashMap::new());
                adapter.sync_scheduled_tick().await.unwrap();
                assert!(listener.contains("remote", "CCCCCCCC"));
                assert_eq!(
                    store.set_attempts.load(Ordering::SeqCst),
                    4,
                    "withdrawn write capability must disable the existing writer"
                );
                readonly_manifest.assert_async().await;
                readonly_child.assert_async().await;
            }
        }
    }
}

// Legacy may publish a remote cursor even if its condensed child was not updated.
// Parseability is not proof that the cached members cover the remote cursor.
#[tokio::test]
async fn stale_legacy_child_is_fully_reconciled_before_incremental_http() {
    for polling in [false, true] {
        for cached_size in [30, 40, 50] {
            for (cached_child, stale_member) in
                [("+AAAAAAAA\n", false), ("+AAAAAAAA\n+ZZZZZZZZ\n", true)]
            {
                let mut server = Server::new_async().await;
                let options = StatsigOptions {
                    id_lists_url: Some(format!("{}/get_id_lists", server.url())),
                    ..Default::default()
                };
                let store = Arc::new(
                    SnapshotDataStore::new(HashMap::new(), polling).with_snapshot_write_support(),
                );
                let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
                    "secret-key",
                    Arc::new(HashUtil::new()),
                    &options,
                    store.clone(),
                ));
                store.set_values(complete_snapshot_values(
                    &adapter,
                    "a",
                    cached_child,
                    cached_size,
                ));
                let listener = Arc::new(TestIdListsListener {
                    id_lists: RwLock::new(HashMap::new()),
                });
                adapter
                    .clone()
                    .start(&StatsigRuntime::get_runtime(), listener.clone())
                    .await
                    .unwrap();
                store.set_values(HashMap::new());
                let download_url = format!("{}/a", server.url());
                let manifest = |size| {
                    serde_json::json!({"a": {
                        "name": "a", "url": download_url, "fileID": "file-1",
                        "size": size, "creationTime": 1
                    }})
                    .to_string()
                };
                let mut source = server
                    .mock("POST", "/get_id_lists")
                    .with_status(200)
                    .with_body(manifest(40))
                    .expect(2)
                    .create_async()
                    .await;
                let failed_download = server
                    .mock("GET", "/a")
                    .match_header("range", "bytes=0-")
                    .with_status(500)
                    .expect(1)
                    .create_async()
                    .await;
                assert!(adapter.sync_scheduled_tick().await.is_err());
                assert!(listener.contains("a", "AAAAAAAA"));
                assert_eq!(listener.contains("a", "ZZZZZZZZ"), stale_member);
                assert_eq!(
                    listener.id_lists.read()["a"].metadata.size,
                    cached_size as u64
                );
                failed_download.assert_async().await;
                failed_download.remove_async().await;
                let mut download = server
                    .mock("GET", "/a")
                    .match_header("range", "bytes=0-")
                    .with_status(206)
                    .with_body("+AAAAAAAA\n+BBBBBBBB\n+CCCCCCCC\n+DDDDDDDD\n")
                    .expect(1)
                    .create_async()
                    .await;
                let result = adapter.sync_scheduled_tick().await;
                download.assert_async().await;
                result.unwrap();
                for id in ["AAAAAAAA", "BBBBBBBB", "CCCCCCCC", "DDDDDDDD"] {
                    assert!(
                        listener.contains("a", id),
                        "missing {id}, polling={polling}, cached_size={cached_size}"
                    );
                }
                assert!(!listener.contains("a", "ZZZZZZZZ"));
                assert_eq!(listener.id_lists.read()["a"].metadata.size, 40);
                source.assert_async().await;
                download.assert_async().await;
                source.remove_async().await;
                download.remove_async().await;
                // SET may have written the complete result to the cache. Force a
                // cache miss without accepting another cache snapshot so this
                // tick exercises the trusted HTTP cursor directly.
                store.set_values(HashMap::new());
                source = server
                    .mock("POST", "/get_id_lists")
                    .with_status(200)
                    .with_body(manifest(50))
                    .expect(1)
                    .create_async()
                    .await;
                download = server
                    .mock("GET", "/a")
                    .match_header("range", "bytes=40-")
                    .with_status(206)
                    .with_body("+EEEEEEEE\n")
                    .expect(1)
                    .create_async()
                    .await;
                adapter.sync_scheduled_tick().await.unwrap();
                assert!(listener.contains("a", "EEEEEEEE"));
                assert!(listener.contains("a", "BBBBBBBB"));
                assert_eq!(listener.id_lists.read()["a"].metadata.size, 50);
                source.assert_async().await;
                download.assert_async().await;
            }
        }
    }
}

#[tokio::test]
async fn get_errors_use_atomic_http_fallback_and_recover_to_cache() {
    for fail_manifest in [true, false] {
        for http_success in [true, false] {
            let mut server = Server::new_async().await;
            let options = StatsigOptions {
                id_lists_url: Some(format!("{}/get_id_lists", server.url())),
                ..Default::default()
            };
            let store = Arc::new(
                SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support(),
            );
            let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
                "secret-key",
                Arc::new(HashUtil::new()),
                &options,
                store.clone(),
            ));
            let mut values = complete_snapshot_values(&adapter, "a", "+AAAAAAAA\n", 30);
            let mut b = complete_snapshot_values(&adapter, "b", "+BBBBBBBB\n", 30);
            let mut manifest: serde_json::Value =
                serde_json::from_str(&values[&adapter.manifest_key]).unwrap();
            let b_manifest: serde_json::Value =
                serde_json::from_str(&b.remove(&adapter.manifest_key).unwrap()).unwrap();
            manifest["b"] = b_manifest["b"].clone();
            values.insert(adapter.manifest_key.clone(), manifest.to_string());
            values.extend(b);
            store.set_values(values.clone());
            let listener = Arc::new(TestIdListsListener {
                id_lists: RwLock::new(HashMap::new()),
            });
            adapter
                .clone()
                .start(&StatsigRuntime::get_runtime(), listener.clone())
                .await
                .unwrap();
            store.child_reads.store(0, Ordering::SeqCst);
            if fail_manifest {
                store.failed_keys.lock().push(adapter.manifest_key.clone());
            } else {
                // Fail after one successful child read, regardless of HashMap order.
                store.fail_child_read_number.store(2, Ordering::SeqCst);
            }
            let request = server
                .mock("POST", "/get_id_lists")
                .with_status(if http_success { 200 } else { 500 })
                .with_body("{}")
                .expect(1)
                .create_async()
                .await;
            let result = adapter.sync_scheduled_tick().await;
            assert_eq!(result.is_ok(), http_success);
            assert_eq!(
                store.child_reads.load(Ordering::SeqCst),
                if fail_manifest { 0 } else { 2 }
            );
            if http_success {
                assert!(listener.id_lists.read().is_empty());
            } else {
                assert!(store.written_keys.lock().is_empty());
                assert!(listener.contains("a", "AAAAAAAA"));
                assert!(listener.contains("b", "BBBBBBBB"));
                for name in ["a", "b"] {
                    assert_eq!(listener.id_lists.read()[name].metadata.size, 30);
                }
            }
            request.assert_async().await;
            store.failed_keys.lock().clear();
            store.fail_child_read_number.store(0, Ordering::SeqCst);
            store.set_values(values.clone());
            let writes_before_cache_reload = store.written_keys.lock().len();
            adapter.sync_scheduled_tick().await.unwrap();
            assert_eq!(
                store.written_keys.lock().len(),
                writes_before_cache_reload,
                "cache reads must not echo writes"
            );
            assert!(listener.contains("a", "AAAAAAAA"));
            assert!(listener.contains("b", "BBBBBBBB"));
            assert_eq!(
                *store.values.lock(),
                values,
                "cache reads must not mutate fixture contents"
            );
        }
    }
}

#[tokio::test]
async fn startup_and_timer_refresh_serialize_and_queued_work_cancels_on_shutdown() {
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let shared = Arc::new(SharedDataStore::new(store.clone()));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &StatsigOptions::default(),
        shared.clone(),
    ));
    store.set_values(complete_snapshot_values(&adapter, "a", "+AAAAAAAA\n", 30));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    let runtime = StatsigRuntime::get_runtime();
    adapter
        .clone()
        .start(&runtime, listener.clone())
        .await
        .unwrap();
    let before = store.requested_keys.lock().len();
    store.set_values(complete_snapshot_values(&adapter, "a", "+BBBBBBBB\n", 40));
    let guard = adapter.polling_task.lock_refresh().await;
    let mut startup = Box::pin(adapter.clone().start(&runtime, listener.clone()));
    let mut tick = Box::pin(adapter.sync_scheduled_tick());
    assert!(futures::poll!(&mut startup).is_pending());
    assert!(futures::poll!(&mut tick).is_pending());
    // Reads can make progress; neither cache installation nor listener rebinding
    // can interleave with an HTTP refresh holding the publication gate.
    assert_eq!(store.requested_keys.lock().len(), before + 2);
    assert!(listener.contains("a", "AAAAAAAA"));
    assert!(!listener.contains("a", "BBBBBBBB"));
    drop(guard);
    tokio::try_join!(startup, tick).unwrap();
    assert_eq!(store.requested_keys.lock().len(), before + 4);
    assert!(listener.contains("a", "BBBBBBBB"));

    let guard = adapter.polling_task.lock_refresh().await;
    let mut startup = Box::pin(adapter.clone().start(&runtime, listener));
    let mut tick = Box::pin(adapter.sync_scheduled_tick());
    assert!(futures::poll!(&mut startup).is_pending());
    assert!(futures::poll!(&mut tick).is_pending());
    let before = store.requested_keys.lock().len();
    let (shutdown, startup, tick) = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(adapter.shutdown(Duration::from_secs(1)), startup, tick)
    })
    .await
    .unwrap();
    shutdown.unwrap();
    assert!(startup.is_err());
    assert!(tick.is_err());
    assert_eq!(store.requested_keys.lock().len(), before);
    shared.shutdown().await.unwrap();
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
    drop(guard);
}

#[tokio::test]
async fn superseded_refresh_cannot_publish_or_fall_through_to_http() {
    let mut server = Server::new_async().await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..Default::default()
    };
    let store =
        Arc::new(SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support());
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(&adapter, "a", "+AAAAAAAA\n", 30));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    let old_epoch = current_runtime_epoch().wrapping_sub(1);
    store.set_values(complete_snapshot_values(&adapter, "a", "+BBBBBBBB\n", 40));
    assert!(matches!(
        adapter.sync_cache_or_http(old_epoch, false).await,
        Err(StatsigErr::ShutdownFailure(_))
    ));
    assert!(listener.contains("a", "AAAAAAAA"));
    store.set_values(HashMap::new());
    assert!(matches!(
        adapter.sync_cache_or_http(old_epoch, false).await,
        Err(StatsigErr::ShutdownFailure(_))
    ));
    assert!(matches!(
        adapter.sync_http(old_epoch, false).await,
        Err(StatsigErr::ShutdownFailure(_))
    ));
    let source = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    // The commit fence runs after the HTTP await, before publication or writes.
    assert!(matches!(
        adapter
            .http_fallback
            .sync_id_lists_with_reconciliation(true, || false)
            .await,
        Err(StatsigErr::ShutdownFailure(_))
    ));
    source.assert_async().await;
    assert!(listener.contains("a", "AAAAAAAA"));
    assert_eq!(listener.id_lists.read()["a"].metadata.size, 30);
    assert!(adapter.needs_http_reconciliation.load(Ordering::Acquire));
    assert!(store.written_keys.lock().is_empty());
}

#[tokio::test]
async fn loads_complete_snapshot_from_exact_manifest_and_child_keys() {
    let options = StatsigOptions::default();
    let hashing = Arc::new(HashUtil::new());
    let empty_store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        hashing,
        &options,
        empty_store.clone(),
    ));
    let remote_log = "+2Tv4fIVX\n+/WGgOvT3\n-/WGgOvT3\n";
    let child = "+2Tv4fIVX\n";
    let values = complete_snapshot_values(&adapter, "paid-users", child, remote_log.len());
    empty_store.set_values(values);
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    assert!(listener.contains("paid-users", "2Tv4fIVX"));
    assert!(!listener.contains("paid-users", "/WGgOvT3"));
    assert_eq!(
        listener.id_lists.read()["paid-users"].metadata.size,
        remote_log.len() as u64
    );
    assert_eq!(
        *empty_store.requested_keys.lock(),
        vec![
            adapter.manifest_key.clone(),
            adapter.child_key("paid-users")
        ]
    );
}

#[tokio::test]
async fn loads_present_empty_legacy_child_with_nonzero_remote_size() {
    let options = StatsigOptions::default();
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let values = complete_snapshot_values(&adapter, "paid-users", "", 42);
    store.set_values(values);
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    let lists = listener.id_lists.read();
    assert!(lists["paid-users"].ids.is_empty());
    assert_eq!(lists["paid-users"].metadata.size, 42);
}

#[tokio::test]
async fn invalid_refresh_preserves_last_complete_snapshot() {
    let options = StatsigOptions::default();
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        "+2Tv4fIVX\n".len(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    store.set_values(HashMap::from([(
        adapter.manifest_key.clone(),
        r#"["replacement"]"#.to_string(),
    )]));

    assert!(adapter.sync_id_lists().await.is_err());
    assert!(listener.contains("paid-users", "2Tv4fIVX"));
    assert!(!listener.contains("replacement", "2Tv4fIVX"));
}

#[tokio::test]
async fn starts_from_cache_even_when_data_store_polling_is_disabled() {
    let options = StatsigOptions {
        disable_network: Some(true),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), false));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    assert!(listener.contains("paid-users", "2Tv4fIVX"));
}

#[tokio::test]
async fn absent_feature_opt_in_uses_http_without_reading_id_list_keys() {
    let mut server = Server::new_async().await;
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true).without_snapshot_support());
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener)
        .await
        .unwrap();

    assert!(!adapter.supports_snapshot());
    assert!(store.requested_keys.lock().is_empty());
    assert!(store.writes.lock().is_empty());
    manifest_request.assert_async().await;
}

#[tokio::test]
async fn failed_store_initialization_falls_back_to_http_without_write_back() {
    let mut server = Server::new_async().await;
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(2)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let store = Arc::new(
        SnapshotDataStore::new(HashMap::new(), true)
            .with_snapshot_write_support()
            .with_failed_initialize(),
    );
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    assert!(adapter.supports_snapshot());
    assert!(store.requested_keys.lock().is_empty());
    assert!(store.writes.lock().is_empty());
    store.fail_initialize.store(false, Ordering::SeqCst);
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    adapter.sync_scheduled_tick().await.unwrap();

    assert!(listener.contains("paid-users", "2Tv4fIVX"));
    // GET already proves read recovery. This slice must also activate the writer
    // after readiness recovers, without allowing writes during the failed init.
    store.set_values(HashMap::new());
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(
        *store.writes.lock(),
        vec![(adapter.manifest_key.clone(), "{}".into())]
    );
    manifest_request.assert_async().await;
}

#[tokio::test]
async fn absent_polling_override_defaults_to_cache_refreshes() {
    let options = StatsigOptions {
        disable_network: Some(true),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), false).without_polling_override());
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+/WGgOvT3\n",
        10,
    ));

    adapter.sync_scheduled_tick().await.unwrap();

    assert!(listener.contains("paid-users", "/WGgOvT3"));
    assert!(!listener.contains("paid-users", "2Tv4fIVX"));
    assert_eq!(store.requested_keys.lock().len(), 4);
}

#[tokio::test]
async fn polling_capable_cache_refreshes_membership_on_repeated_ticks_without_http() {
    let mut server = Server::new_async().await;
    let no_http = server
        .mock("POST", "/get_id_lists")
        .expect(0)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        id_lists_sync_interval_ms: Some(20),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        9,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    let runtime = StatsigRuntime::get_runtime();
    adapter
        .clone()
        .start(&runtime, listener.clone())
        .await
        .unwrap();
    adapter
        .clone()
        .schedule_background_sync(&runtime)
        .await
        .unwrap();

    for member in ["/WGgOvT3", "AAAAAAAA"] {
        store.set_values(complete_snapshot_values(
            &adapter,
            "paid-users",
            &format!("+{member}\n"),
            9,
        ));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while !listener.contains("paid-users", member) && tokio::time::Instant::now() < deadline {
            sleep(Duration::from_millis(10)).await;
        }
        assert!(listener.contains("paid-users", member), "missing {member}");
    }

    adapter.shutdown(Duration::from_secs(1)).await.unwrap();
    no_http.assert_async().await;
    assert!(store.requested_keys.lock().len() >= 6);
}

#[tokio::test]
async fn shutdown_cancels_in_flight_id_list_poll_before_shared_store_shutdown() {
    let options = StatsigOptions {
        id_lists_sync_interval_ms: Some(1),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let shared = Arc::new(SharedDataStore::new(store.clone()));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        shared.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        9,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    let runtime = StatsigRuntime::get_runtime();
    adapter.clone().start(&runtime, listener).await.unwrap();
    store.block_next_get.store(true, Ordering::SeqCst);
    adapter
        .clone()
        .schedule_background_sync(&runtime)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), store.get_started.notified())
        .await
        .unwrap();

    tokio::time::timeout(
        Duration::from_secs(1),
        adapter.shutdown(Duration::from_secs(1)),
    )
    .await
    .unwrap()
    .unwrap();
    shared.shutdown().await.unwrap();
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn shutdown_cancels_in_flight_id_list_write_before_shared_store_shutdown() {
    let mut server = Server::new_async().await;
    let manifest = format!(
        r#"{{"remote":{{"name":"remote","url":"{}/remote","fileID":"file-1","size":10,"creationTime":1}}}}"#,
        server.url()
    );
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body(manifest)
        .expect(1)
        .create_async()
        .await;
    let child_request = server
        .mock("GET", "/remote")
        .with_status(200)
        .with_body("+2Tv4fIVX\n")
        .expect(1)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_sync_interval_ms: Some(1),
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let store =
        Arc::new(SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support());
    let shared = Arc::new(SharedDataStore::new(store.clone()));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        shared.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    let runtime = StatsigRuntime::get_runtime();
    adapter
        .clone()
        .start(&runtime, listener.clone())
        .await
        .unwrap();
    store.set_values(HashMap::from([(
        adapter.manifest_key.clone(),
        "{invalid".to_string(),
    )]));
    store.block_next_set.store(true, Ordering::SeqCst);
    adapter
        .clone()
        .schedule_background_sync(&runtime)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), store.set_started.notified())
        .await
        .unwrap();

    store.set_values(complete_snapshot_values(
        &adapter,
        "cached",
        "+BBBBBBBB\n",
        30,
    ));
    let mut queued_cache = Box::pin(adapter.sync_scheduled_tick());
    assert!(futures::poll!(&mut queued_cache).is_pending());
    assert!(listener.contains("remote", "2Tv4fIVX"));
    assert!(!listener.contains("cached", "BBBBBBBB"));

    tokio::time::timeout(Duration::from_secs(1), async {
        let (shutdown, queued_cache) =
            tokio::join!(adapter.shutdown(Duration::from_secs(1)), queued_cache);
        shutdown.unwrap();
        assert!(queued_cache.is_err());
        Ok::<(), StatsigErr>(())
    })
    .await
    .unwrap()
    .unwrap();
    shared.shutdown().await.unwrap();
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
    assert!(store.writes.lock().is_empty());
    manifest_request.assert_async().await;
    child_request.assert_async().await;
}

#[tokio::test]
async fn invalid_startup_cache_falls_back_to_a_whole_http_refresh() {
    let mut server = Server::new_async().await;
    let manifest = format!(
        r#"{{"remote":{{"name":"remote","url":"{}/remote","fileID":"file-1","size":10,"creationTime":1}}}}"#,
        server.url()
    );
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body(manifest)
        .expect(1)
        .create_async()
        .await;
    let child_request = server
        .mock("GET", "/remote")
        .with_status(200)
        .with_body("+2Tv4fIVX\n")
        .expect(1)
        .create_async()
        .await;
    let store =
        Arc::new(SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support());
    let options = StatsigOptions {
        data_store: Some(store.clone()),
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    assert!(listener.contains("remote", "2Tv4fIVX"));
    {
        let writes = store.writes.lock();
        assert_eq!(writes.len(), 2);
        assert!(writes[0].0.contains("id_list::remote"));
        assert!(writes[1].0.contains("/v1/get_id_lists"));
    }
    adapter.sync_scheduled_tick().await.unwrap();
    assert!(listener.contains("remote", "2Tv4fIVX"));
    manifest_request.assert_async().await;
    child_request.assert_async().await;
}

#[tokio::test]
async fn writer_checkpoint_skips_unchanged_http_and_resets_after_cache_or_capability_changes() {
    let mut server = Server::new_async().await;
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body(
            serde_json::json!({"remote": {
                "name": "remote", "url": format!("{}/remote", server.url()),
                "fileID": "file-1", "size": 10, "creationTime": 1
            }})
            .to_string(),
        )
        .expect(8)
        .create_async()
        .await;
    let child_request = server
        .mock("GET", "/remote")
        .with_status(206)
        .with_body("+AAAAAAAA\n")
        .expect(2)
        .create_async()
        .await;
    let store =
        Arc::new(SnapshotDataStore::new(HashMap::new(), false).with_snapshot_write_support());
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..Default::default()
    };
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 2);
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(
        store.set_attempts.load(Ordering::SeqCst),
        2,
        "unchanged tick must not write"
    );

    store.supports_snapshot_write.store(false, Ordering::SeqCst);
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 2);
    store.supports_snapshot_write.store(true, Ordering::SeqCst);
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(
        store.set_attempts.load(Ordering::SeqCst),
        4,
        "re-enabled writer must republish"
    );

    // A cache snapshot can replace the IDs while retaining the same cursor.
    store.set_values(complete_snapshot_values(
        &adapter,
        "remote",
        "+BBBBBBBB\n",
        10,
    ));
    adapter.sync_id_lists().await.unwrap();
    assert!(listener.contains("remote", "BBBBBBBB"));
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 6);
    assert!(listener.contains("remote", "AAAAAAAA"));
    assert!(!listener.contains("remote", "BBBBBBBB"));
    assert_eq!(
        store.values.lock()[&adapter.child_key("remote")],
        "+AAAAAAAA\n"
    );
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 6);

    // A missing cache must be repaired even though remote/in-memory state agrees.
    store.set_values(HashMap::new());
    adapter
        .sync_cache_or_http(current_runtime_epoch(), false)
        .await
        .unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 8);
    adapter.sync_scheduled_tick().await.unwrap();
    assert_eq!(store.set_attempts.load(Ordering::SeqCst), 8);
    manifest_request.assert_async().await;
    child_request.assert_async().await;
}

#[tokio::test]
async fn write_failure_does_not_discard_a_valid_http_update() {
    let mut server = Server::new_async().await;
    let manifest = format!(
        r#"{{"remote":{{"name":"remote","url":"{}/remote","fileID":"file-1","size":10,"creationTime":1}}}}"#,
        server.url()
    );
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body(manifest)
        .expect(3)
        .create_async()
        .await;
    let child_request = server
        .mock("GET", "/remote")
        .with_status(200)
        .with_body("+2Tv4fIVX\n")
        .expect(1)
        .create_async()
        .await;
    let store =
        Arc::new(SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support());
    store.fail_writes.store(true, Ordering::SeqCst);
    let options = StatsigOptions {
        data_store: Some(store.clone()),
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });

    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();

    assert!(listener.contains("remote", "2Tv4fIVX"));
    assert!(store.writes.lock().is_empty());
    assert!(store.set_attempts.load(Ordering::SeqCst) > 0);
    store.fail_writes.store(false, Ordering::SeqCst);
    adapter
        .sync_http(current_runtime_epoch(), false)
        .await
        .unwrap();
    assert_eq!(
        store.writes.lock().len(),
        2,
        "unchanged HTTP must retry failed persistence"
    );
    let attempts = store.set_attempts.load(Ordering::SeqCst);
    adapter
        .sync_http(current_runtime_epoch(), false)
        .await
        .unwrap();
    assert_eq!(
        store.set_attempts.load(Ordering::SeqCst),
        attempts,
        "successful retry establishes checkpoint"
    );
    manifest_request.assert_async().await;
    child_request.assert_async().await;
}

#[tokio::test]
async fn polling_cache_failure_falls_back_to_http_for_that_tick() {
    let mut server = Server::new_async().await;
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        10,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    store.set_values(HashMap::from([(
        adapter.manifest_key.clone(),
        "{invalid".to_string(),
    )]));

    adapter.sync_scheduled_tick().await.unwrap();

    assert!(!listener.contains("paid-users", "2Tv4fIVX"));
    manifest_request.assert_async().await;
}

#[tokio::test]
async fn non_polling_store_uses_http_directly_after_cache_first_startup() {
    let mut server = Server::new_async().await;
    let manifest_request = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let options = StatsigOptions {
        id_lists_url: Some(format!("{}/get_id_lists", server.url())),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), false));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        9,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    assert!(listener.contains("paid-users", "2Tv4fIVX"));
    let reads_after_start = store.requested_keys.lock().len();

    adapter.sync_scheduled_tick().await.unwrap();

    assert!(!listener.contains("paid-users", "2Tv4fIVX"));
    assert_eq!(store.requested_keys.lock().len(), reads_after_start);
    manifest_request.assert_async().await;
}

#[tokio::test]
async fn failed_cache_and_http_refresh_keeps_last_known_good() {
    let options = StatsigOptions {
        disable_network: Some(true),
        ..StatsigOptions::default()
    };
    let store = Arc::new(SnapshotDataStore::new(HashMap::new(), true));
    let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
        "secret-key",
        Arc::new(HashUtil::new()),
        &options,
        store.clone(),
    ));
    store.set_values(complete_snapshot_values(
        &adapter,
        "paid-users",
        "+2Tv4fIVX\n",
        9,
    ));
    let listener = Arc::new(TestIdListsListener {
        id_lists: RwLock::new(HashMap::new()),
    });
    adapter
        .clone()
        .start(&StatsigRuntime::get_runtime(), listener.clone())
        .await
        .unwrap();
    store.set_values(HashMap::from([(
        adapter.manifest_key.clone(),
        "{invalid".to_string(),
    )]));

    let error = adapter.sync_scheduled_tick().await.unwrap_err();

    assert!(error.to_string().contains("HTTP fallback failed"));
    assert!(listener.contains("paid-users", "2Tv4fIVX"));
}

fn two_list_remote_manifest(server: &Server) -> String {
    serde_json::json!({
        "a": {"name":"a", "url":format!("{}/a", server.url()), "fileID":"new-a", "size":10, "creationTime":2},
        "b": {"name":"b", "url":format!("{}/b", server.url()), "fileID":"new-b", "size":10, "creationTime":2},
    }).to_string()
}

#[tokio::test]
async fn incomplete_http_startup_is_atomic_only_for_opted_in_snapshots() {
    // DataStore snapshots remain atomic; unsupported stores use HTTP's partial
    // publication while initialization still reports the failed download.
    for (supported, initialize_fails) in [(true, false), (true, true), (false, false)] {
        let mut server = Server::new_async().await;
        let manifest = server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(two_list_remote_manifest(&server))
            .expect(1)
            .create_async()
            .await;
        let a = server
            .mock("GET", "/a")
            .with_status(200)
            .with_body("+2Tv4fIVX\n")
            .expect_at_most(1)
            .create_async()
            .await;
        let b = server
            .mock("GET", "/b")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", server.url())),
            ..Default::default()
        };
        let mut store = SnapshotDataStore::new(HashMap::new(), true).with_snapshot_write_support();
        store.supports_snapshot = supported;
        store
            .fail_initialize
            .store(initialize_fails, Ordering::SeqCst);
        let store = Arc::new(store);
        let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
            "secret-key",
            Arc::new(HashUtil::new()),
            &options,
            store.clone(),
        ));
        let listener = Arc::new(TestIdListsListener {
            id_lists: RwLock::new(HashMap::new()),
        });
        assert!(adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .is_err());
        assert!(
            listener.contains("a", "2Tv4fIVX") == !supported,
            "snapshot opt-in={supported}, initializer failure={initialize_fails}"
        );
        assert!(!listener.contains("b", "2Tv4fIVX"));
        assert!(
            store.writes.lock().is_empty(),
            "a partial candidate must never be written"
        );
        manifest.assert_async().await;
        a.assert_async().await;
        b.assert_async().await;
        if !supported {
            manifest.remove_async().await;
            a.remove_async().await;
            b.remove_async().await;
            let mut next_manifest: serde_json::Value =
                serde_json::from_str(&two_list_remote_manifest(&server)).unwrap();
            next_manifest["a"]["size"] = 20.into();
            let next_source = server
                .mock("POST", "/get_id_lists")
                .with_status(200)
                .with_body(next_manifest.to_string())
                .expect(1)
                .create_async()
                .await;
            let next_a = server
                .mock("GET", "/a")
                .match_header("range", "bytes=10-")
                .with_status(206)
                .with_body("+CCCCCCCC\n")
                .expect(1)
                .create_async()
                .await;
            let next_b = server
                .mock("GET", "/b")
                .with_status(500)
                .expect(1)
                .create_async()
                .await;
            adapter.sync_scheduled_tick().await.unwrap();
            assert!(listener.contains("a", "CCCCCCCC"));
            assert!(store.requested_keys.lock().is_empty());
            next_source.assert_async().await;
            next_a.assert_async().await;
            next_b.assert_async().await;
        }
    }
}

#[tokio::test]
async fn incomplete_http_refresh_preserves_all_memberships_and_cursors_until_complete() {
    for polling in [true, false] {
        let mut server = Server::new_async().await;
        let manifest = server
            .mock("POST", "/get_id_lists")
            .with_status(200)
            .with_body(two_list_remote_manifest(&server))
            .expect(2)
            .create_async()
            .await;
        let a = server
            .mock("GET", "/a")
            .with_status(200)
            .with_body("+KiHsugjY\n")
            .expect_at_least(1)
            .expect_at_most(2)
            .create_async()
            .await;
        let b_failed = server
            .mock("GET", "/b")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let options = StatsigOptions {
            id_lists_url: Some(format!("{}/get_id_lists", server.url())),
            ..Default::default()
        };
        let store =
            Arc::new(SnapshotDataStore::new(HashMap::new(), polling).with_snapshot_write_support());
        let adapter = Arc::new(StatsigDataStoreIdListsAdapter::new(
            "secret-key",
            Arc::new(HashUtil::new()),
            &options,
            store.clone(),
        ));
        let mut values = HashMap::new();
        let mut entries = serde_json::Map::new();
        for name in ["a", "b", "c"] {
            let mut list = complete_snapshot_values(&adapter, name, "+2Tv4fIVX\n", 10);
            let entry: serde_json::Value =
                serde_json::from_str(&list.remove(&adapter.manifest_key).unwrap()).unwrap();
            entries.insert(name.into(), entry[name].clone());
            values.extend(list);
        }
        values.insert(
            adapter.manifest_key.clone(),
            serde_json::Value::Object(entries).to_string(),
        );
        store.set_values(values);
        let listener = Arc::new(TestIdListsListener {
            id_lists: RwLock::new(HashMap::new()),
        });
        adapter
            .clone()
            .start(&StatsigRuntime::get_runtime(), listener.clone())
            .await
            .unwrap();
        let snapshot = || {
            listener
                .id_lists
                .read()
                .iter()
                .map(|(name, list)| {
                    (
                        name.clone(),
                        (
                            serde_json::to_value(&list.metadata).unwrap(),
                            list.ids.clone(),
                        ),
                    )
                })
                .collect::<HashMap<_, _>>()
        };
        let before = snapshot();
        // Keep the cache invalid through both ticks, or the second tick would bypass HTTP.
        store.set_values(HashMap::new());
        assert!(adapter.sync_scheduled_tick().await.is_err());
        assert_eq!(
            snapshot(),
            before,
            "partial fallback must preserve all A/B/C metadata and IDs (polling={polling})"
        );
        assert!(
            store.writes.lock().is_empty(),
            "failed A/B must not publish children or manifest"
        );
        b_failed.assert_async().await;
        b_failed.remove_async().await;
        let b = server
            .mock("GET", "/b")
            .with_status(200)
            .with_body("+KiHsugjY\n")
            .expect(1)
            .create_async()
            .await;
        adapter.sync_scheduled_tick().await.unwrap();
        assert_eq!(listener.id_lists.read().len(), 2);
        for name in ["a", "b"] {
            assert!(listener.contains(name, "KiHsugjY"));
            assert!(!listener.contains(name, "2Tv4fIVX"));
            assert_eq!(listener.id_lists.read()[name].metadata.size, 10);
        }
        assert!(!listener.id_lists.read().contains_key("c"));
        let writes = store.writes.lock().clone();
        assert_eq!(writes.len(), 3);
        assert_eq!(writes[0].0, adapter.child_key("a"));
        assert_eq!(writes[1].0, adapter.child_key("b"));
        assert_eq!(writes[2].0, adapter.manifest_key);
        let published: serde_json::Value = serde_json::from_str(&writes[2].1).unwrap();
        assert!(published.get("c").is_none());
        assert_eq!(published["b"]["fileID"], "new-b");
        manifest.assert_async().await;
        a.assert_async().await;
        b.assert_async().await;
    }
}
