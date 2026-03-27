use super::config_spec_background_sync_metrics::log_config_sync_overall_latency;
use super::response_format::get_specs_response_format;
use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{DataStoreBytesResponse, DataStoreTrait, RequestPath};
use crate::networking::ResponseData;
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::{
    log_d, log_e, log_w, read_lock_or_else, unwrap_or_else, write_lock_or_else, SpecsAdapter,
    SpecsUpdateListener,
};
use crate::{StatsigErr, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use tokio::time::{self, sleep};

const TAG: &str = "StatsigDataStoreSpecsAdapter";

pub struct StatsigDataStoreSpecsAdapter {
    data_store: Arc<dyn DataStoreTrait>,
    cache_key: String,
    sync_interval: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
}

impl StatsigDataStoreSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        data_store_key: &str,
        data_store: Arc<dyn DataStoreTrait>,
        options: Option<&StatsigOptions>,
    ) -> Self {
        let default_options = StatsigOptions::default();
        let options_ref = options.unwrap_or(&default_options);

        StatsigDataStoreSpecsAdapter {
            data_store,
            cache_key: data_store_key.to_string(),
            sync_interval: Duration::from_millis(u64::from(
                options_ref
                    .specs_sync_interval_ms
                    .unwrap_or(DEFAULT_SYNC_INTERVAL_MS),
            )),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    async fn load_cached_specs_bytes(&self) -> Result<DataStoreBytesResponse, StatsigErr> {
        // `get_bytes()` falls back to `get()` by default, so legacy string-backed data stores
        // continue to work while bytes-capable implementations can override it.
        self.data_store.get_bytes(&self.cache_key).await
    }

    fn binary_specs_response_data_from_bytes(bytes: Vec<u8>) -> ResponseData {
        ResponseData::from_bytes_with_headers(
            bytes,
            Some(HashMap::from([
                (
                    "content-type".to_string(),
                    "application/octet-stream".to_string(),
                ),
                ("content-encoding".to_string(), "statsig-br".to_string()),
            ])),
        )
    }

    async fn execute_background_sync(&self, rt_shutdown_notify: &Arc<Notify>) {
        loop {
            tokio::select! {
                () = sleep(self.sync_interval) => self.execute_background_sync_impl().await,
                () = rt_shutdown_notify.notified() => {
                    log_d!(TAG, "Runtime shutdown. Shutting down specs background sync");
                    break;
                }
                () = self.shutdown_notify.notified() => {
                    log_d!(TAG, "Shutting down specs background sync");
                    break;
                }
            }
        }
    }

    async fn execute_background_sync_impl(&self) {
        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        let update = match self.load_cached_specs_bytes().await {
            Ok(update) => update,
            Err(e) => {
                log_w!(TAG, "Failed to read for data store: {e}");
                return;
            }
        };

        let read_lock = read_lock_or_else!(self.listener, {
            log_w!(TAG, "Unable to acquire read lock on listener");
            return;
        });

        let listener = unwrap_or_else!(read_lock.as_ref(), {
            log_w!(TAG, "Listener not set");
            return;
        });

        let data = if self.data_store.supports_bytes() {
            Self::binary_specs_response_data_from_bytes(update.result.unwrap_or_default())
        } else {
            ResponseData::from_bytes(update.result.unwrap_or_default())
        };

        let response_format = get_specs_response_format(&data);
        let result = listener.did_receive_specs_update(SpecsUpdate {
            data,
            source: SpecsSource::Adapter("DataStore".to_string()),
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some("datastore".to_string()),
        });
        log_config_sync_overall_latency(
            &self.ops_stats,
            sync_start_ms,
            "datastore",
            response_format.as_str(),
            false,
            result.is_ok(),
            result
                .as_ref()
                .err()
                .map_or_else(String::new, |e| e.to_string()),
            false,
        );

        if let Err(e) = result {
            log_w!(TAG, "Failed to send specs update to listener: {e}");
        }
    }

    // --------- END - Observability helpers ---------
}

#[async_trait]
impl SpecsAdapter for StatsigDataStoreSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        self.data_store.initialize().await?;

        let update = self.load_cached_specs_bytes().await?;

        let data = match update.result {
            Some(data) => data,
            None => return Err(StatsigErr::DataStoreFailure("Empty result".to_string())),
        };

        let read_lock = read_lock_or_else!(self.listener, {
            return Err(StatsigErr::UnstartedAdapter(
                "Failed to acquire read lock on listener".to_string(),
            ));
        });

        let listener = match read_lock.as_ref() {
            Some(listener) => listener,
            None => return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
        };

        let data = if self.data_store.supports_bytes() {
            Self::binary_specs_response_data_from_bytes(data)
        } else {
            ResponseData::from_bytes(data)
        };
        let response_format = get_specs_response_format(&data);

        let result = listener.did_receive_specs_update(SpecsUpdate {
            data,
            source: SpecsSource::Adapter("DataStore".to_string()),
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some("datastore".to_string()),
        });
        log_config_sync_overall_latency(
            &self.ops_stats,
            sync_start_ms,
            "datastore",
            response_format.as_str(),
            false,
            result.is_ok(),
            result
                .as_ref()
                .err()
                .map_or_else(String::new, |e| e.to_string()),
            false,
        );
        result
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        let mut write_lock = write_lock_or_else!(self.listener, {
            log_e!(TAG, "Failed to acquire write lock on listener");
            return;
        });

        *write_lock = Some(listener);
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // Support polling updates function should be pretty cheap. But we have to make it async
        let should_schedule = self
            .data_store
            .support_polling_updates_for(RequestPath::RulesetsV2)
            .await;

        if !should_schedule {
            return Err(StatsigErr::SpecsAdapterSkipPoll(self.get_type_name()));
        }

        let weak_self = Arc::downgrade(&self);

        statsig_runtime.spawn(
            "data_store_specs_adapter",
            move |rt_shutdown_notify| async move {
                let strong_self = if let Some(strong_self) = weak_self.upgrade() {
                    strong_self
                } else {
                    log_w!(TAG, "Failed to upgrade weak instance");
                    return;
                };

                strong_self
                    .execute_background_sync(&rt_shutdown_notify)
                    .await;
            },
        )?;

        Ok(())
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();
        time::timeout(timeout, async { self.data_store.shutdown().await })
            .await
            .map_err(|e| StatsigErr::DataStoreFailure(format!("Failed to shutdown: {e}")))?
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigDataStoreSpecAdapter).to_string()
    }
}
