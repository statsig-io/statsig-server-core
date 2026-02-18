use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{DataStoreTrait, RequestPath};
use crate::networking::ResponseData;
use crate::{
    log_d, log_e, log_w, read_lock_or_else, unwrap_or_else, write_lock_or_else, SpecsAdapter,
    SpecsUpdateListener,
};
use crate::{StatsigErr, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use tokio::time::{self, sleep};

const TAG: &str = "StatsigDataStoreSpecsAdapter";

pub struct StatsigDataStoreSpecsAdapter {
    data_store: Arc<dyn DataStoreTrait>,
    cache_key: String,
    sync_interval: Duration,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
}

impl StatsigDataStoreSpecsAdapter {
    pub fn new(
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
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
        }
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
        let update = match self.data_store.get(&self.cache_key).await {
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

        if let Err(e) = listener.did_receive_specs_update(SpecsUpdate {
            data: ResponseData::from_bytes(update.result.unwrap_or_default().into_bytes()),
            source: SpecsSource::Adapter("DataStore".to_string()),
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some("datastore".to_string()),
        }) {
            log_w!(TAG, "Failed to send specs update to listener: {e}");
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigDataStoreSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.data_store.initialize().await?;

        let update = self.data_store.get(&self.cache_key).await?;

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

        listener.did_receive_specs_update(SpecsUpdate {
            data: ResponseData::from_bytes(data.into_bytes()),
            source: SpecsSource::Adapter("DataStore".to_string()),
            received_at: Utc::now().timestamp_millis() as u64,
            source_api: Some("datastore".to_string()),
        })
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
