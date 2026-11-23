use super::{
    id_lists_snapshot::IdListsSnapshot, statsig_http_id_lists_adapter::StatsigHttpIdListsAdapter,
    IdListsAdapter, IdListsUpdateListener,
};
use crate::data_store_interface::{
    get_id_list_data_store_key, get_id_lists_manifest_data_store_key, DataStoreFeature,
    DataStoreTrait,
};
use crate::data_store_lifecycle::DataStorePollingTask;
use crate::hashing::HashUtil;
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::sdk_diagnostics::diagnostics::ContextType;
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker, StepType};
use crate::statsig_global::current_runtime_epoch;
use crate::{log_w, StatsigErr, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time::sleep;

const TAG: &str = "StatsigDataStoreIdListsAdapter";
const DEFAULT_ID_LIST_SYNC_INTERVAL_MS: u32 = 60_000;
const DATA_STORE_SOURCE: &str = "Adapter(DataStore)";

/// Reads complete ID-list snapshots from a DataStore, with an HTTP fallback.
///
/// A shared lifecycle wrapper initializes the store once across config specs
/// and ID lists. A scheduled tick uses the feature-specific polling preference.
pub struct StatsigDataStoreIdListsAdapter {
    data_store: Arc<dyn DataStoreTrait>,
    sdk_key: String,
    hashing: Arc<HashUtil>,
    options: StatsigOptions,
    manifest_key: String,
    listener: RwLock<Option<Arc<dyn IdListsUpdateListener>>>,
    sync_interval_duration: Duration,
    polling_task: DataStorePollingTask,
    http_fallback: StatsigHttpIdListsAdapter,
    snapshot_supported: AtomicBool,
    snapshot_write_supported: AtomicBool,
    // Legacy manifest cursors do not prove that condensed children are complete.
    needs_http_reconciliation: AtomicBool,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl StatsigDataStoreIdListsAdapter {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        hashing: Arc<HashUtil>,
        options: &StatsigOptions,
        data_store: Arc<dyn DataStoreTrait>,
    ) -> Self {
        let manifest_key = get_id_lists_manifest_data_store_key(sdk_key, &hashing, options);

        Self {
            data_store,
            sdk_key: sdk_key.to_string(),
            hashing,
            options: options.clone(),
            manifest_key,
            listener: RwLock::new(None),
            sync_interval_duration: Duration::from_millis(u64::from(
                options
                    .id_lists_sync_interval_ms
                    .unwrap_or(DEFAULT_ID_LIST_SYNC_INTERVAL_MS),
            )),
            polling_task: DataStorePollingTask::new(),
            http_fallback: StatsigHttpIdListsAdapter::new(sdk_key, options),
            snapshot_supported: AtomicBool::new(false),
            snapshot_write_supported: AtomicBool::new(false),
            needs_http_reconciliation: AtomicBool::new(false),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        }
    }

    pub fn force_shutdown(&self) {
        self.polling_task.stop();
    }

    pub fn supports_snapshot(&self) -> bool {
        self.snapshot_supported.load(Ordering::Acquire)
    }

    async fn initialize_store_if_needed(&self, epoch: u64) -> Result<(), StatsigErr> {
        // SharedDataStore serializes initialization and caches only success.
        let initialized = self.data_store.initialize().await;
        let write_supported = initialized.is_ok()
            && self
                .data_store
                .supports_feature(DataStoreFeature::IDListSnapshotWrite)
                .await;
        if self.polling_task.is_stopped_in_epoch(epoch) {
            return Err(StatsigErr::ShutdownFailure(
                "ID-list writer setup superseded".into(),
            ));
        }
        self.snapshot_write_supported
            .store(write_supported, Ordering::Release);
        initialized
    }

    fn add_diagnostics_marker(
        &self,
        context: ContextType,
        action: ActionType,
        step: StepType,
        success: Option<bool>,
    ) {
        let mut marker = Marker::new(KeyType::GetIDListSources, action, Some(step))
            .with_source(DATA_STORE_SOURCE.to_string());
        if let Some(success) = success {
            marker = marker.with_is_success(success);
        }
        self.ops_stats.add_marker(marker, context);
    }

    fn add_read_end_marker(&self, context: ContextType, error: Option<&StatsigErr>) {
        let mut marker = Marker::new(
            KeyType::GetIDListSources,
            ActionType::End,
            Some(StepType::DataStoreRead),
        )
        .with_source(DATA_STORE_SOURCE.to_string())
        .with_is_success(error.is_none());
        if let Some(error) = error {
            marker = marker.with_statsig_error(error);
        }
        self.ops_stats.add_marker(marker, context);
    }

    /// Store initialization runs before the snapshot read span. Its markers are
    /// only added when it fails, so a failure that falls back to HTTP is still
    /// visible without adding a span to every successful sync.
    async fn initialize_store_with_diagnostics(
        &self,
        epoch: u64,
        context: ContextType,
    ) -> Result<(), StatsigErr> {
        let start_marker = Marker::new(
            KeyType::GetIDListSources,
            ActionType::Start,
            Some(StepType::DataStoreRead),
        )
        .with_source(DATA_STORE_SOURCE.to_string());
        let result = self.initialize_store_if_needed(epoch).await;
        if let Err(error) = &result {
            if !matches!(error, StatsigErr::ShutdownFailure(_)) {
                self.ops_stats.add_marker(start_marker, context);
                self.add_read_end_marker(context, Some(error));
            }
        }
        result
    }

    fn child_key(&self, list_name: &str) -> String {
        get_id_list_data_store_key(list_name, &self.sdk_key, &self.hashing, &self.options)
    }

    #[cfg(test)]
    async fn sync_id_lists(&self) -> Result<(), StatsigErr> {
        self.sync_id_lists_in_epoch(current_runtime_epoch(), ContextType::ConfigSync)
            .await
    }

    async fn sync_id_lists_in_epoch(
        &self,
        epoch: u64,
        diagnostics_context: ContextType,
    ) -> Result<(), StatsigErr> {
        self.add_diagnostics_marker(
            diagnostics_context,
            ActionType::Start,
            StepType::DataStoreRead,
            None,
        );
        let read_result = self.read_snapshot_from_store().await;
        self.add_read_end_marker(diagnostics_context, read_result.as_ref().err());
        let (manifest, children) = read_result?;

        self.add_diagnostics_marker(
            diagnostics_context,
            ActionType::Start,
            StepType::Process,
            None,
        );
        let result = self.publish_snapshot(epoch, &manifest, &children).await;
        self.add_diagnostics_marker(
            diagnostics_context,
            ActionType::End,
            StepType::Process,
            Some(result.is_ok()),
        );
        result
    }

    async fn read_snapshot_from_store(
        &self,
    ) -> Result<(String, HashMap<String, String>), StatsigErr> {
        let manifest = self
            .data_store
            .get(&self.manifest_key)
            .await?
            .result
            .ok_or_else(|| StatsigErr::DataStoreFailure("ID-list manifest is empty".to_string()))?;
        let mut children = HashMap::new();

        for list_name in IdListsSnapshot::list_names(&manifest)? {
            let child = self
                .data_store
                .get(&self.child_key(&list_name))
                .await?
                .result
                .ok_or_else(|| {
                    StatsigErr::DataStoreFailure(format!(
                        "ID-list child snapshot is empty for '{list_name}'"
                    ))
                })?;
            children.insert(list_name, child);
        }

        Ok((manifest, children))
    }

    async fn publish_snapshot(
        &self,
        epoch: u64,
        manifest: &str,
        children: &HashMap<String, String>,
    ) -> Result<(), StatsigErr> {
        let snapshot = IdListsSnapshot::parse(manifest, children)?;
        // Reads may overlap a slow public retry. Only publication must exclude
        // HTTP cursor selection/download/write-back and listener rebinding.
        let _refresh = self.polling_task.lock_refresh().await;
        if self.polling_task.is_stopped_in_epoch(epoch) {
            return Err(StatsigErr::ShutdownFailure(
                "ID-list snapshot superseded".into(),
            ));
        }
        let listener = self.listener.read().as_ref().cloned().ok_or_else(|| {
            StatsigErr::UnstartedAdapter("ID-list listener is not set".to_string())
        })?;
        listener.replace_id_lists_snapshot(snapshot.lists)?;
        // The cache may replace members without changing its cursor. Recreate
        // the writer checkpoint after the next full HTTP reconciliation.
        self.http_fallback.disable_data_store_writer();
        self.needs_http_reconciliation
            .store(true, Ordering::Release);
        Ok(())
    }

    async fn sync_http(&self, epoch: u64, initializing: bool) -> Result<(), StatsigErr> {
        let _refresh = self.polling_task.lock_refresh().await;
        if self.polling_task.is_stopped_in_epoch(epoch) {
            return Err(StatsigErr::ShutdownFailure(
                "ID-list HTTP refresh superseded".into(),
            ));
        }
        // Configure at the serialized HTTP boundary, not before cache reads:
        // a slow HTTP refresh must not prevent concurrent cache I/O.
        if self.supports_snapshot() && self.snapshot_write_supported.load(Ordering::Acquire) {
            self.http_fallback.enable_data_store_writer(
                &self.sdk_key,
                &self.options,
                self.data_store.clone(),
            );
        } else {
            self.http_fallback.disable_data_store_writer();
        }
        let reconcile = self.needs_http_reconciliation.load(Ordering::Acquire);
        let should_publish = || !self.polling_task.is_stopped_in_epoch(epoch);
        if self.supports_snapshot() {
            self.http_fallback
                .sync_id_lists_with_reconciliation(reconcile, initializing, should_publish)
                .await?;
        } else {
            // Unsupported stores remain HTTP-only, including partial-success
            // publication, while retaining the shutdown/epoch publication fence.
            self.http_fallback
                .sync_id_lists_with_publication_check(initializing, should_publish)
                .await?;
        }
        // A failed candidate must leave both the installed snapshot and this
        // requirement intact, so the next attempt cannot skip missing members.
        self.needs_http_reconciliation
            .store(false, Ordering::Release);
        Ok(())
    }

    fn fallback_error(cache_error: StatsigErr, http_error: StatsigErr, stage: &str) -> StatsigErr {
        if matches!(http_error, StatsigErr::ShutdownFailure(_)) {
            return http_error;
        }
        StatsigErr::DataStoreFailure(format!(
            "DataStore ID-list {stage} failed: {cache_error}; HTTP fallback failed: {http_error}"
        ))
    }

    async fn sync_cache_or_http(&self, epoch: u64, initializing: bool) -> Result<(), StatsigErr> {
        let diagnostics_context = if initializing {
            ContextType::Initialize
        } else {
            ContextType::ConfigSync
        };
        match self
            .sync_id_lists_in_epoch(epoch, diagnostics_context)
            .await
        {
            Ok(()) => Ok(()),
            Err(cache_error) => {
                if self.polling_task.is_stopped_in_epoch(epoch) {
                    return Err(StatsigErr::ShutdownFailure(
                        "ID-list refresh superseded".into(),
                    ));
                }
                {
                    let _refresh = self.polling_task.lock_refresh().await;
                    // Missing/corrupt cache data must be repaired even if HTTP
                    // returns the manifest this writer previously persisted.
                    self.http_fallback.disable_data_store_writer();
                }
                log_w!(
                    TAG,
                    "DataStore ID-list refresh failed: {cache_error}; trying HTTP"
                );
                self.sync_http(epoch, initializing)
                    .await
                    .map_err(|http_error| Self::fallback_error(cache_error, http_error, "snapshot"))
            }
        }
    }

    async fn sync_scheduled_tick(&self) -> Result<(), StatsigErr> {
        let epoch = current_runtime_epoch();
        let (mut stop, completion) = self.polling_task.begin()?;
        if *stop.borrow() {
            return Err(StatsigErr::ShutdownFailure(
                "ID-list adapter stopped".into(),
            ));
        }
        tokio::select! {
          biased;
          _ = stop.changed() => Err(StatsigErr::ShutdownFailure("ID-list refresh cancelled".into())),
          result = async {
        if completion.is_stopped() || current_runtime_epoch() != epoch {
            return Err(StatsigErr::ShutdownFailure("ID-list refresh superseded".into()));
        }
        if !self.supports_snapshot() {
            return self.sync_http(epoch, false).await;
        }

        if let Err(cache_error) = self
            .initialize_store_with_diagnostics(epoch, ContextType::ConfigSync)
            .await
        {
            return self.sync_http(epoch, false).await.map_err(|http_error| Self::fallback_error(cache_error, http_error, "initialization"));
        }

        if self
            .data_store
            .support_polling_updates_for_feature(DataStoreFeature::IDListSnapshot)
            .await
            .unwrap_or(true)
        {
            self.sync_cache_or_http(epoch, false).await
        } else {
            self.sync_http(epoch, false).await
        }
          } => result,
        }
    }

    async fn run_background_sync(weak_self: &Weak<Self>) {
        let Some(adapter) = weak_self.upgrade() else {
            return;
        };

        if let Err(error) = adapter.sync_scheduled_tick().await {
            log_w!(TAG, "ID-list refresh failed: {error}");
        }
        adapter
            .ops_stats
            .enqueue_diagnostics_event(Some(KeyType::GetIDListSources), ContextType::ConfigSync);
    }

    fn schedule_background_sync_impl(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        mut immediate: bool,
    ) -> Result<(), StatsigErr> {
        let weak_self = Arc::downgrade(&self);
        let interval_duration = self.sync_interval_duration;
        let Some((mut stop, completion)) = self.polling_task.begin_loop()? else {
            return Ok(());
        };
        statsig_runtime.spawn(
            "data_store_id_list_bg_sync",
            move |rt_shutdown_notify| async move {
                let _completion = completion;
                loop {
                    if *stop.borrow() {
                        break;
                    }
                    if !immediate {
                        tokio::select! {
                            biased;
                            _ = stop.changed() => break,
                            () = rt_shutdown_notify.notified() => break,
                            () = sleep(interval_duration) => {},
                        }
                    }
                    immediate = false;
                    tokio::select! {
                        biased;
                        _ = stop.changed() => break,
                        () = rt_shutdown_notify.notified() => break,
                        () = Self::run_background_sync(&weak_self) => {},
                    }
                }
            },
        )?;
        Ok(())
    }
}

#[async_trait]
impl IdListsAdapter for StatsigDataStoreIdListsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        // Public retries and the timer share cancellation/draining with shutdown.
        let epoch = current_runtime_epoch();
        let (mut stop, completion) = self.polling_task.begin()?;
        if *stop.borrow() {
            return Err(StatsigErr::ShutdownFailure(
                "ID-list adapter stopped".into(),
            ));
        }
        tokio::select! {
          biased;
          _ = stop.changed() => Err(StatsigErr::ShutdownFailure("ID-list startup cancelled by shutdown".into())),
          result = async {
        {
        let _refresh = self.polling_task.lock_refresh().await;
        if completion.is_stopped() || current_runtime_epoch() != epoch {
            return Err(StatsigErr::ShutdownFailure("ID-list startup superseded".into()));
        }
        *self.listener.write() = Some(listener.clone());
        self.http_fallback.set_listener(listener);
        }
        let supported = self
            .data_store
            .supports_feature(DataStoreFeature::IDListSnapshot)
            .await;
        if self.polling_task.is_stopped_in_epoch(epoch) {
            return Err(StatsigErr::ShutdownFailure("ID-list startup superseded".into()));
        }
        self.snapshot_supported.store(supported, Ordering::Release);
        if !supported {
            self.snapshot_write_supported.store(false, Ordering::Release);
            return self.sync_http(epoch, true).await;
        }
        if let Err(cache_error) = self
            .initialize_store_with_diagnostics(epoch, ContextType::Initialize)
            .await
        {
            log_w!(
                TAG,
                "DataStore ID-list initialization failed: {cache_error}; trying HTTP"
            );
            return self.sync_http(epoch, true).await.map_err(|http_error| Self::fallback_error(cache_error, http_error, "initialization"));
        }
        self.sync_cache_or_http(epoch, true).await
          } => result,
        }
    }

    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        self.polling_task.stop_and_wait().await;
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.schedule_background_sync_impl(statsig_runtime, false)
    }

    async fn schedule_background_sync_after_fork(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.schedule_background_sync_impl(statsig_runtime, true)
    }

    fn get_type_name(&self) -> String {
        TAG.to_string()
    }
}

#[cfg(test)]
#[path = "statsig_data_store_id_lists_adapter_tests.rs"]
mod tests;
