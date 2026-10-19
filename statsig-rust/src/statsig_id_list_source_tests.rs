use super::*;
use crate::data_store_interface::{
    DataStoreFeature, DataStoreResponse, DataStoreTrait, RequestPath,
};
use crate::specs_adapter::{SpecAdapterConfig, SpecsAdapterType};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

struct NoopDataStore;

struct FailingSpecs;

#[async_trait]
impl SpecsAdapter for FailingSpecs {
    fn initialize(&self, _listener: Arc<dyn SpecsUpdateListener>) {}

    async fn start(self: Arc<Self>, _rt: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Err(StatsigErr::DataStoreFailure("specs-only failure".into()))
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        _rt: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _rt: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        "FailingSpecs".into()
    }
}

#[tokio::test]
async fn specs_failure_does_not_make_ready_id_list_initialization_unsuccessful() {
    let client = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            data_store: Some(Arc::new(StrictModeDataStore {
                supports_polling: true,
                id_list_manifest: Some("{}".into()),
            })),
            specs_adapter: Some(Arc::new(FailingSpecs)),
            enable_id_lists: Some(true),
            disable_all_logging: Some(true),
            disable_country_lookup: Some(true),
            disable_network: Some(true),
            ..Default::default()
        })),
    );
    let _ = client.initialize().await;
    let details = client.get_initialize_details();
    assert!(details.init_success);
    assert_eq!(details.is_id_list_ready, Some(true));
    assert!(details
        .failure_details
        .as_ref()
        .unwrap()
        .reason
        .contains("specs-only failure"));
    client.shutdown().await.unwrap();
}

struct BlockingShutdownDataStore {
    started: Notify,
    release: Notify,
    shutdowns: AtomicUsize,
}

#[async_trait]
impl DataStoreTrait for BlockingShutdownDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        self.release.notified().await;
        Ok(())
    }

    async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
        unreachable!()
    }

    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        unreachable!()
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        false
    }
}

#[tokio::test]
async fn timed_out_sdk_shutdown_does_not_repeat_shared_store_shutdown() {
    let store = Arc::new(BlockingShutdownDataStore {
        started: Notify::new(),
        release: Notify::new(),
        shutdowns: AtomicUsize::new(0),
    });
    let statsig = Arc::new(Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            data_store: Some(store.clone()),
            disable_all_logging: Some(true),
            disable_network: Some(true),
            ..StatsigOptions::default()
        })),
    ));
    statsig
        .shared_data_store
        .as_ref()
        .unwrap()
        .initialize()
        .await
        .unwrap();

    let first = tokio::spawn({
        let statsig = statsig.clone();
        async move {
            statsig
                .shutdown_with_timeout(Duration::from_millis(100))
                .await
        }
    });
    tokio::time::timeout(Duration::from_secs(1), store.started.notified())
        .await
        .unwrap();
    assert!(matches!(
        first.await.unwrap(),
        Err(StatsigErr::ShutdownFailure(_))
    ));
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);

    store.release.notify_one();
    statsig
        .shutdown_with_timeout(Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(store.shutdowns.load(Ordering::SeqCst), 1);
}

struct StrictModeDataStore {
    supports_polling: bool,
    id_list_manifest: Option<String>,
}

#[async_trait]
impl DataStoreTrait for NoopDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
        Ok(DataStoreResponse {
            result: None,
            time: None,
        })
    }

    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        false
    }
}

#[async_trait]
impl DataStoreTrait for StrictModeDataStore {
    async fn supports_feature(&self, feature: DataStoreFeature) -> bool {
        matches!(
            feature,
            DataStoreFeature::ConfigSpecs | DataStoreFeature::IDListSnapshot
        )
    }

    async fn support_polling_updates_for_feature(&self, feature: DataStoreFeature) -> Option<bool> {
        (feature == DataStoreFeature::IDListSnapshot).then_some(self.supports_polling)
    }

    async fn initialize(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        let result = if key.contains("/v2/download_config_specs") {
            Some(include_str!("../tests/data/dcs_missing_gates.json").to_string())
        } else if key.contains("/v1/get_id_lists") {
            self.id_list_manifest.clone()
        } else {
            None
        };

        Ok(DataStoreResponse {
            result,
            time: Some(1),
        })
    }

    async fn set(&self, _key: &str, _value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
        self.supports_polling
    }
}

#[test]
fn data_store_selects_capability_routing_id_list_adapter() {
    let options = StatsigOptions {
        data_store: Some(Arc::new(NoopDataStore)),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    };

    let housing = initialize_id_lists_adapter("secret-key", Arc::new(HashUtil::new()), &options);

    assert!(housing.inner.is_some());
    assert!(housing.as_data_store_adapter.is_some());
}

#[test]
fn no_data_store_selects_http_id_list_adapter() {
    let options = StatsigOptions {
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    };

    let housing = initialize_id_lists_adapter("secret-key", Arc::new(HashUtil::new()), &options);

    assert!(housing.inner.is_some());
    assert!(housing.as_default_adapter.is_some());
    assert!(housing.as_data_store_adapter.is_none());
}

#[test]
fn data_store_capability_is_resolved_at_startup() {
    let options = StatsigOptions {
        data_store: Some(Arc::new(NoopDataStore)),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    };

    let housing = initialize_id_lists_adapter("secret-key", Arc::new(HashUtil::new()), &options);

    assert!(housing.inner.is_some());
    assert!(housing.as_default_adapter.is_none());
    assert!(housing.as_data_store_adapter.is_some());
}

#[test]
fn custom_id_list_adapter_takes_precedence() {
    let options = StatsigOptions {
        data_store: Some(Arc::new(NoopDataStore)),
        enable_id_lists: Some(true),
        id_lists_adapter: Some(Arc::new(StatsigHttpIdListsAdapter::new(
            "secret-key",
            &StatsigOptions::default(),
        ))),
        ..StatsigOptions::default()
    };

    let housing = initialize_id_lists_adapter("secret-key", Arc::new(HashUtil::new()), &options);

    assert!(housing.inner.is_some());
    assert!(housing.as_default_adapter.is_none());
    assert!(housing.as_data_store_adapter.is_none());
}

#[test]
fn custom_specs_adapter_can_coexist_with_id_list_store() {
    let options = StatsigOptions {
        data_store: Some(Arc::new(NoopDataStore)),
        enable_id_lists: Some(true),
        spec_adapters_config: Some(vec![SpecAdapterConfig {
            adapter_type: SpecsAdapterType::NetworkHttp,
            init_timeout_ms: 0,
            specs_url: Some("http://localhost:9999/v2/download_config_specs".to_string()),
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            domain_name: None,
        }]),
        ..StatsigOptions::default()
    };

    let id_lists_housing =
        initialize_id_lists_adapter("secret-key", Arc::new(HashUtil::new()), &options);
    assert!(id_lists_housing.inner.is_some());
    assert!(id_lists_housing.as_data_store_adapter.is_some());
}

#[tokio::test]
async fn data_store_and_http_failure_is_visible_on_public_initialization() {
    let options = Arc::new(StatsigOptions {
        data_store: Some(Arc::new(StrictModeDataStore {
            supports_polling: true,
            id_list_manifest: None,
        })),
        disable_all_logging: Some(true),
        disable_country_lookup: Some(true),
        disable_network: Some(true),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    });
    let client = Statsig::new("secret-key", Some(options));

    let error = client.initialize().await.unwrap_err();
    assert!(
        matches!(error, StatsigErr::DataStoreFailure(message) if message.contains("manifest is empty") && message.contains("HTTP fallback failed"))
    );

    let details = client.get_initialize_details();
    assert!(!details.init_success);
    assert_eq!(details.is_id_list_ready, Some(false));
    assert!(details.failure_details.is_some());
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn no_data_store_uses_permissive_http_initialization() {
    let options = Arc::new(StatsigOptions {
        disable_all_logging: Some(true),
        disable_country_lookup: Some(true),
        disable_network: Some(true),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    });
    let client = Statsig::new("secret-key", Some(options));

    client.initialize().await.unwrap();

    let details = client.get_initialize_details();
    assert!(details.init_success);
    assert_eq!(details.is_id_list_ready, Some(false));
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_cache_and_http_failure_is_visible_on_public_initialization() {
    let options = Arc::new(StatsigOptions {
        data_store: Some(Arc::new(StrictModeDataStore {
            supports_polling: true,
            id_list_manifest: Some("{not-json".to_string()),
        })),
        disable_all_logging: Some(true),
        disable_country_lookup: Some(true),
        disable_network: Some(true),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    });
    let client = Statsig::new("secret-key", Some(options));

    let error = client.initialize().await.unwrap_err();
    assert!(
        matches!(error, StatsigErr::DataStoreFailure(message) if message.contains("Invalid ID-list manifest") && message.contains("HTTP fallback failed"))
    );

    let details = client.get_initialize_details();
    assert!(!details.init_success);
    assert_eq!(details.is_id_list_ready, Some(false));
    assert!(details.failure_details.is_some());
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn snapshot_startup_does_not_require_polling_support() {
    let options = Arc::new(StatsigOptions {
        data_store: Some(Arc::new(StrictModeDataStore {
            supports_polling: false,
            id_list_manifest: Some("{}".to_string()),
        })),
        disable_all_logging: Some(true),
        disable_country_lookup: Some(true),
        disable_network: Some(true),
        enable_id_lists: Some(true),
        ..StatsigOptions::default()
    });
    let client = Statsig::new("secret-key", Some(options));

    client.initialize().await.unwrap();

    let details = client.get_initialize_details();
    assert!(details.init_success);
    assert_eq!(details.is_id_list_ready, Some(true));
    client.shutdown().await.unwrap();
}
