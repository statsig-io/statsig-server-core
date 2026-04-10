use super::config_spec_background_sync_metrics::log_config_sync_overall_latency;
use super::response_format::get_specs_response_format;
use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{
    DataStoreBytesResponse, DataStoreCacheKeys, DataStoreResponse, DataStoreTrait, RequestPath,
};
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
    cache_keys: DataStoreCacheKeys,
    sync_interval: Duration,
    ops_stats: Arc<OpsStatsForInstance>,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
}

struct CachedSpecs {
    result: Option<Vec<u8>>,
    is_protobuf: bool,
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
            cache_keys: DataStoreCacheKeys::from_selected_key(data_store_key),
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
}

#[async_trait]
impl SpecsAdapter for StatsigDataStoreSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let sync_start_ms = Utc::now().timestamp_millis() as u64;
        self.data_store.initialize().await?;

        let update = self.load_cached_specs().await?;
        if update.result.is_none() {
            return Err(StatsigErr::DataStoreFailure("Empty result".to_string()));
        }

        let read_lock = read_lock_or_else!(self.listener, {
            return Err(StatsigErr::UnstartedAdapter(
                "Failed to acquire read lock on listener".to_string(),
            ));
        });

        let listener = match read_lock.as_ref() {
            Some(listener) => listener,
            None => return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
        };

        let (result, response_format) = self.send_specs_update_to_listener(listener, update);
        self.log_data_store_sync_result(sync_start_ms, &response_format, &result);
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

impl StatsigDataStoreSpecsAdapter {
    async fn load_cached_specs(&self) -> Result<CachedSpecs, StatsigErr> {
        if let Some(update) = self.load_statsig_br_cache().await? {
            return Ok(update);
        }

        self.load_plain_text_cache().await
    }

    async fn load_statsig_br_cache(&self) -> Result<Option<CachedSpecs>, StatsigErr> {
        match self
            .load_cached_specs_bytes(&self.cache_keys.statsig_br, true)
            .await
        {
            Ok(update) => Ok(update),
            Err(e @ StatsigErr::BytesNotImplemented) => {
                self.load_cached_specs_string(Some(e)).await.map(Some)
            }
            Err(e) => {
                log_w!(
                    TAG,
                    "Failed to read statsig-br specs bytes from data store. Trying plain text cache: {}",
                    e
                );
                Ok(None)
            }
        }
    }

    async fn load_plain_text_cache(&self) -> Result<CachedSpecs, StatsigErr> {
        match self
            .load_cached_specs_bytes(&self.cache_keys.plain_text, false)
            .await
        {
            Ok(Some(update)) => Ok(update),
            Ok(None) => Ok(CachedSpecs {
                result: None,
                is_protobuf: false,
            }),
            Err(e @ StatsigErr::BytesNotImplemented) => {
                self.load_cached_specs_string(Some(e)).await
            }
            Err(e) => Err(e),
        }
    }

    async fn load_cached_specs_bytes(
        &self,
        key: &str,
        is_protobuf: bool,
    ) -> Result<Option<CachedSpecs>, StatsigErr> {
        let response = self.data_store.get_bytes(key).await?;
        Ok(cached_specs_from_bytes_response(response, is_protobuf))
    }

    async fn load_cached_specs_string(
        &self,
        bytes_error: Option<StatsigErr>,
    ) -> Result<CachedSpecs, StatsigErr> {
        if let Some(e) = bytes_error {
            log_w!(
                TAG,
                "Failed to read specs from data store as bytes. Falling back to string read: {}",
                e
            );
        }

        let response = self.data_store.get(&self.cache_keys.plain_text).await?;
        Ok(cached_specs_from_string_response(response))
    }

    fn specs_response_data_from_cache(data: Vec<u8>, is_protobuf: bool) -> ResponseData {
        if is_protobuf {
            return Self::binary_specs_response_data_from_bytes(data);
        }

        ResponseData::from_bytes(data)
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

    fn send_specs_update_to_listener(
        &self,
        listener: &Arc<dyn SpecsUpdateListener>,
        cached_specs: CachedSpecs,
    ) -> (Result<(), StatsigErr>, String) {
        let data = Self::specs_response_data_from_cache(
            cached_specs.result.unwrap_or_default(),
            cached_specs.is_protobuf,
        );
        let response_format = get_specs_response_format(&data);

        let result = listener.did_receive_specs_update(SpecsUpdate {
            data,
            source: SpecsSource::Adapter("DataStore".to_string()),
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some("datastore".to_string()),
        });

        (result, response_format.as_str().to_string())
    }

    fn log_data_store_sync_result(
        &self,
        sync_start_ms: u64,
        response_format: &str,
        result: &Result<(), StatsigErr>,
    ) {
        log_config_sync_overall_latency(
            &self.ops_stats,
            sync_start_ms,
            "datastore",
            response_format,
            false,
            result.is_ok(),
            result
                .as_ref()
                .err()
                .map_or_else(String::new, |e| e.to_string()),
            false,
        );
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
        let update = match self.load_cached_specs().await {
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

        let (result, response_format) = self.send_specs_update_to_listener(listener, update);
        self.log_data_store_sync_result(sync_start_ms, &response_format, &result);

        if let Err(e) = result {
            log_w!(TAG, "Failed to send specs update to listener: {e}");
        }
    }
}

fn cached_specs_from_bytes_response(
    response: DataStoreBytesResponse,
    is_protobuf: bool,
) -> Option<CachedSpecs> {
    response.result.map(|result| CachedSpecs {
        result: Some(result),
        is_protobuf,
    })
}

fn cached_specs_from_string_response(response: DataStoreResponse) -> CachedSpecs {
    CachedSpecs {
        result: response.result.map(String::into_bytes),
        is_protobuf: false,
    }
}
