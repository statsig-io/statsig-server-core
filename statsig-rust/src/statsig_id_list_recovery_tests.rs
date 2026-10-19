use super::*;
use crate::data_store_interface::{
    DataStoreFeature, DataStoreResponse, DataStoreTrait, RequestPath,
};
use crate::specs_adapter::StatsigBootstrapSpecsAdapter;
use async_trait::async_trait;
use mockito::Server;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Default)]
struct RecoveringStore {
    child: Mutex<Option<String>>,
    fail_initialize: AtomicBool,
    ready: AtomicBool,
    initialize_calls: AtomicUsize,
    initialized: AtomicUsize,
    block_initialize: AtomicBool,
    initializing: AtomicBool,
    initialize_started: Notify,
    block_next_read: AtomicBool,
    reading: AtomicBool,
    read_started: Notify,
    release_read: Notify,
    shutdowns: AtomicUsize,
}

struct Initializing<'a>(&'a AtomicBool);
impl Drop for Initializing<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl RecoveringStore {
    fn member(&self, user: &str) {
        let hash = HashUtil::new().sha256(user);
        *self.child.lock() = Some(format!("+{}\n", &hash[..8]));
    }
}

#[async_trait]
impl DataStoreTrait for RecoveringStore {
    async fn supports_feature(&self, feature: DataStoreFeature) -> bool {
        feature == DataStoreFeature::IDListSnapshot
    }
    async fn initialize(&self) -> Result<(), StatsigErr> {
        assert!(!self.initializing.swap(true, Ordering::SeqCst));
        let _active = Initializing(&self.initializing);
        self.initialize_calls.fetch_add(1, Ordering::SeqCst);
        if self.block_initialize.load(Ordering::SeqCst) {
            self.initialize_started.notify_one();
            std::future::pending::<()>().await;
        }
        if self.fail_initialize.load(Ordering::SeqCst) {
            return Err(StatsigErr::DataStoreFailure("backend unavailable".into()));
        }
        self.ready.store(true, Ordering::SeqCst);
        self.initialized.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        assert!(
            self.ready.load(Ordering::SeqCst),
            "read before initialization"
        );
        if self.block_next_read.swap(false, Ordering::SeqCst) {
            self.reading.store(true, Ordering::SeqCst);
            let _active = Initializing(&self.reading);
            self.read_started.notify_one();
            self.release_read.notified().await;
        }
        let child = self.child.lock().clone();
        let result = if key.contains("/get_id_lists") {
            child.map(|body| serde_json::json!({"list_a": {
                "name": "list_a", "size": body.len(), "url": "", "fileID": "file-a", "creationTime": 1
            }}).to_string())
        } else {
            child
        };
        Ok(DataStoreResponse {
            result,
            time: Some(1),
        })
    }
    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        Ok(())
    }
    async fn shutdown(&self) -> Result<(), StatsigErr> {
        assert!(!self.initializing.load(Ordering::SeqCst));
        assert!(!self.reading.load(Ordering::SeqCst));
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        false
    }
}

struct CountingSpecs {
    bootstrap: Arc<StatsigBootstrapSpecsAdapter>,
    scheduled: AtomicUsize,
    fail_shutdown: AtomicBool,
}

#[async_trait]
impl SpecsAdapter for CountingSpecs {
    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        self.bootstrap.initialize(listener);
    }
    async fn start(self: Arc<Self>, rt: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        self.bootstrap.clone().start(rt).await
    }
    async fn schedule_background_sync(
        self: Arc<Self>,
        _rt: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.scheduled.fetch_add(1, Ordering::SeqCst);
        // Make concurrent initialize calls cross the old load/store window.
        tokio::task::yield_now().await;
        Ok(())
    }
    async fn shutdown(
        &self,
        _timeout: Duration,
        _rt: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        if self.fail_shutdown.load(Ordering::SeqCst) {
            return Err(StatsigErr::ShutdownFailure("specs shutdown failed".into()));
        }
        Ok(())
    }
    fn get_type_name(&self) -> String {
        "CountingSpecs".into()
    }
}

fn client(
    store: Arc<RecoveringStore>,
    server: &mockito::ServerGuard,
) -> (Arc<Statsig>, Arc<CountingSpecs>) {
    let specs = serde_json::json!({
        "has_updates": true, "time": 1, "dynamic_configs": {}, "layer_configs": {}, "experiment_to_layer": {},
        "feature_gates": {"membership": {
            "name": "membership", "type": "feature_gate", "entity": "feature_gate", "salt": "s", "enabled": true,
            "defaultValue": false, "idType": "userID", "rules": [{
                "id": "r", "name": "r", "salt": "s", "passPercentage": 100, "returnValue": true,
                "idType": "userID", "conditions": ["member"]
            }]
        }},
        "condition_map": {"member": {"type": "unit_id", "operator": "in_segment_list", "targetValue": "list_a", "idType": "userID"}}
    });
    let specs = Arc::new(CountingSpecs {
        bootstrap: Arc::new(StatsigBootstrapSpecsAdapter::new(specs.to_string())),
        scheduled: AtomicUsize::new(0),
        fail_shutdown: AtomicBool::new(false),
    });
    let statsig = Arc::new(Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            data_store: Some(store),
            specs_adapter: Some(specs.clone()),
            enable_id_lists: Some(true),
            id_lists_sync_interval_ms: Some(1000),
            id_lists_url: Some(format!("{}/get_id_lists", server.url())),
            disable_all_logging: Some(true),
            disable_country_lookup: Some(true),
            ..StatsigOptions::default()
        })),
    ));
    (statsig, specs)
}

async fn wait_for_member(client: &Statsig, user: &str) {
    let user = StatsigUser::with_user_id(user);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !client.check_gate(&user, "membership") {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("polling did not recover membership");
}

fn assert_one_poller(client: &Statsig) {
    assert_eq!(
        client
            .statsig_runtime
            .get_running_task_ids()
            .iter()
            .filter(|(tag, _)| tag == "data_store_id_list_bg_sync")
            .count(),
        1
    );
}

#[tokio::test]
#[serial]
async fn request_failure_recovers_without_application_retry_and_keeps_initial_error() {
    let mut server = Server::new_async().await;
    let failed_http = server
        .mock("POST", "/get_id_lists")
        .with_status(500)
        .create_async()
        .await;
    let store = Arc::new(RecoveringStore::default());
    let (client, specs) = client(store.clone(), &server);
    assert!(client.initialize().await.is_err());
    assert!(!client.get_initialize_details().init_success);
    assert_eq!(
        client.get_initialize_details().is_id_list_ready,
        Some(false)
    );
    let original_details = serde_json::to_value(client.get_initialize_details()).unwrap();
    store.member("first");
    wait_for_member(&client, "first").await;
    store.member("second");
    wait_for_member(&client, "second").await;
    assert!(!client.check_gate(&StatsigUser::with_user_id("first"), "membership"));
    assert!(!client.get_initialize_details().init_success);
    assert_eq!(
        serde_json::to_value(client.get_initialize_details()).unwrap(),
        original_details
    );
    assert_eq!(specs.scheduled.load(Ordering::SeqCst), 1);
    assert_eq!(store.initialized.load(Ordering::SeqCst), 1);
    assert_one_poller(&client);
    client.shutdown().await.unwrap();
    failed_http.assert_async().await;
}

#[tokio::test]
#[serial]
async fn initializer_failure_http_success_then_timer_uses_recovered_store() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let store = Arc::new(RecoveringStore::default());
    store.fail_initialize.store(true, Ordering::SeqCst);
    let (client, _) = client(store.clone(), &server);
    client.initialize().await.unwrap();
    store.member("recovered");
    store.fail_initialize.store(false, Ordering::SeqCst);
    wait_for_member(&client, "recovered").await;
    store.member("later");
    wait_for_member(&client, "later").await;
    assert_eq!(store.initialized.load(Ordering::SeqCst), 1);
    assert_one_poller(&client);
    client.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn concurrent_initialization_does_not_duplicate_background_setup() {
    let server = Server::new_async().await;
    let store = Arc::new(RecoveringStore::default());
    store.member("included");
    let (client, specs) = client(store.clone(), &server);
    let (a, b) = tokio::join!(client.initialize(), client.initialize());
    a.unwrap();
    b.unwrap();
    assert_eq!(specs.scheduled.load(Ordering::SeqCst), 1);
    client.initialize().await.unwrap();
    assert_eq!(specs.scheduled.load(Ordering::SeqCst), 1);
    assert_eq!(store.initialized.load(Ordering::SeqCst), 1);
    assert_one_poller(&client);
    client.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn explicit_retry_overlaps_poller_without_adding_loops() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/get_id_lists")
        .with_status(500)
        .expect(1)
        .create_async()
        .await;
    let store = Arc::new(RecoveringStore::default());
    let (client, specs) = client(store.clone(), &server);
    assert!(client.initialize().await.is_err());
    store.member("first");
    wait_for_member(&client, "first").await;
    store.block_next_read.store(true, Ordering::SeqCst);
    let retry = tokio::spawn({
        let client = client.clone();
        async move { client.initialize().await }
    });
    tokio::time::timeout(Duration::from_secs(1), store.read_started.notified())
        .await
        .unwrap();
    store.member("second");
    wait_for_member(&client, "second").await;
    store.release_read.notify_one();
    retry.await.unwrap().unwrap();
    assert!(client.get_initialize_details().init_success);
    assert_eq!(specs.scheduled.load(Ordering::SeqCst), 1);
    assert_eq!(store.initialized.load(Ordering::SeqCst), 1);
    assert_one_poller(&client);
    client.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn shutdown_cancels_blocked_readiness_and_overlapping_public_retry() {
    let mut server = Server::new_async().await;
    let fallback = server
        .mock("POST", "/get_id_lists")
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let store = Arc::new(RecoveringStore::default());
    store.fail_initialize.store(true, Ordering::SeqCst);
    let (client, _) = client(store.clone(), &server);
    client.initialize().await.unwrap();
    store.block_initialize.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), store.initialize_started.notified())
        .await
        .unwrap();
    let retry = tokio::spawn({
        let client = client.clone();
        async move { client.initialize().await }
    });
    tokio::task::yield_now().await;
    client
        .shutdown_with_timeout(Duration::from_secs(1))
        .await
        .unwrap();
    assert!(matches!(
        retry.await.unwrap(),
        Err(StatsigErr::ShutdownFailure(_))
    ));
    assert!(!store.initializing.load(Ordering::SeqCst));
    assert_eq!(store.initialized.load(Ordering::SeqCst), 0);
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 0);
    fallback.assert_async().await;
}

#[derive(Default)]
struct DeferredDrain(AtomicBool);

#[async_trait]
impl crate::id_lists_adapter::IdListsAdapter for DeferredDrain {
    async fn start(
        self: Arc<Self>,
        _rt: &Arc<StatsigRuntime>,
        _listener: Arc<dyn crate::id_lists_adapter::IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }
    async fn schedule_background_sync(
        self: Arc<Self>,
        _rt: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }
    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        // Force a pending first poll: a sibling Result::Err must not drop this drain.
        tokio::task::yield_now().await;
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn get_type_name(&self) -> String {
        "DeferredDrain".into()
    }
}

#[tokio::test]
#[serial]
async fn sibling_shutdown_error_does_not_skip_draining_id_lists() {
    let store = Arc::new(RecoveringStore::default());
    let drain = Arc::new(DeferredDrain::default());
    let specs = Arc::new(CountingSpecs {
        bootstrap: Arc::new(StatsigBootstrapSpecsAdapter::new(
            include_str!("../tests/data/dcs_missing_gates.json").into(),
        )),
        scheduled: AtomicUsize::new(0),
        fail_shutdown: AtomicBool::new(true),
    });
    let client = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            data_store: Some(store.clone()),
            specs_adapter: Some(specs),
            id_lists_adapter: Some(drain.clone()),
            enable_id_lists: Some(true),
            disable_all_logging: Some(true),
            disable_country_lookup: Some(true),
            ..Default::default()
        })),
    );
    client.initialize().await.unwrap();
    client
        .shared_data_store
        .as_ref()
        .unwrap()
        .initialize()
        .await
        .unwrap();
    let error = client.shutdown().await.unwrap_err();
    assert!(error.to_string().contains("specs shutdown failed"));
    assert!(
        drain.0.load(Ordering::SeqCst),
        "sibling error skipped ID-list drain"
    );
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
    // A repeated shutdown may still report the sibling failure, but must not
    // repeat ownership of the successfully shut down shared backend.
    assert!(client.shutdown().await.is_err());
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
}
