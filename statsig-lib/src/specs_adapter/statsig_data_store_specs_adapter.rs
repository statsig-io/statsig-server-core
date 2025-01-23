use std::sync::RwLock;
use std::{sync::Arc, time::Duration};
use tokio::time::{self, sleep};

use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{
    get_data_adapter_key, CompressFormat, DataStoreTrait, RequestPath,
};
use crate::hashing::HashUtil;
use crate::{log_d, log_e, log_w, SpecsAdapter, SpecsUpdateListener};
use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::Notify;

const TAG: &str = stringify!(StatsigDataStoreSpecAdapter);

pub struct StatsigDataStoreSpecsAdapter {
    data_adapter: Arc<dyn DataStoreTrait>,
    cache_key: String,
    sync_interval: Duration,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    shutdown_notify: Arc<Notify>,
}

impl StatsigDataStoreSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        data_adapter: Arc<dyn DataStoreTrait>,
        hashing: &HashUtil,
        sync_interval: Option<u32>,
    ) -> Self {
        let cache_key = get_data_adapter_key(
            RequestPath::RulesetsV2,
            CompressFormat::PlainText,
            &hashing.hash(&sdk_key.to_string(), &crate::HashAlgorithm::Sha256),
        );
        StatsigDataStoreSpecsAdapter {
            data_adapter,
            cache_key,
            sync_interval: Duration::from_millis(
                sync_interval.unwrap_or(DEFAULT_SYNC_INTERVAL_MS) as u64
            ),
            listener: RwLock::new(None),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    async fn execute_background_sync(&self, rt_shutdown_notify: &Arc<Notify>) {
        loop {
            tokio::select! {
                _ = sleep(self.sync_interval) => {
                    let update = self.data_adapter.get(&self.cache_key).await;
                    match update {
                        Ok(update) => {
                        match self.listener.read() {
                            Ok(maybe_listener) => {
                            match maybe_listener.as_ref() {
                                Some(listener) => {
                                    match listener.did_receive_specs_update(SpecsUpdate {
                                        data: update.result.unwrap_or_default(),
                                        source: SpecsSource::Adapter("DataStore".to_string()),
                                        received_at: Utc::now().timestamp_millis() as u64,
                                }) {
                                    Ok(_) => {},
                                    Err(_) => log_w!(TAG, "DataStoreAdapter - Failed to capture"),
                                }},
                                _ => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                            }
                            },
                            _ => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                            }
                        },
                        Err(_) => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                    }
                }
                _ = rt_shutdown_notify.notified() => {
                    log_d!(TAG, "Runtime shutdown. Shutting down specs background sync");
                    break;
                }
                _ = self.shutdown_notify.notified() => {
                    log_d!(TAG, "Shutting down specs background sync");
                    break;
                }
            }
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigDataStoreSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.data_adapter.initialize().await?;
        let update = self.data_adapter.get(&self.cache_key).await?;
        match update.result {
            Some(data) => match &self.listener.read() {
                Ok(read_lock) => match read_lock.as_ref() {
                    Some(listener) => {
                        listener.did_receive_specs_update(SpecsUpdate {
                            data,
                            source: SpecsSource::Adapter("DataStore".to_string()),
                            received_at: Utc::now().timestamp_millis() as u64,
                        })?;
                    }
                    None => {
                        return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string()))
                    }
                },
                Err(_) => return Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            None => return Err(StatsigErr::DataStoreFailure("Empty result".to_string())),
        }
        Ok(())
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on listener: {}", e);
            }
        }
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // Support polling updates function should be pretty cheap. But we have to make it async
        let should_schedule = tokio::task::block_in_place(|| {
            statsig_runtime.get_handle().block_on(async {
                self.data_adapter
                    .support_polling_updates_for(RequestPath::RulesetsV2)
                    .await
            })
        });

        if !should_schedule {
            return Err(StatsigErr::DataStoreSkipPoll);
        }

        let weak_self = Arc::downgrade(&self);

        statsig_runtime.spawn(
            "data_adapter_spec_adapter",
            move |rt_shutdown_notify| async move {
                let strong_self = match weak_self.upgrade() {
                    Some(strong_self) => strong_self,
                    None => {
                        log_w!(TAG, "Failed to upgrade weak instance");
                        return;
                    }
                };

                strong_self
                    .execute_background_sync(&rt_shutdown_notify)
                    .await;
            },
        );

        Ok(())
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();
        time::timeout(timeout, async { self.data_adapter.shutdown().await })
            .await
            .map_err(|e| StatsigErr::DataStoreFailure(format!("Failed to shutdown: {}", e)))?
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigDatStoreSpecAdapter).to_string()
    }
}
