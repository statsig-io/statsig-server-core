use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{
    get_data_adapter_key, CompressFormat, DataStoreTrait, RequestPath,
};
use crate::hashing::HashUtil;
use crate::networking::ResponseData;
use crate::{log_d, log_e, log_w, SpecsAdapter, SpecsUpdateListener};
use crate::{StatsigErr, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use tokio::time::{self, sleep};

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
        options: Option<&StatsigOptions>,
    ) -> Self {
        let cache_key = get_data_adapter_key(
            RequestPath::RulesetsV2,
            CompressFormat::PlainText,
            &hashing.hash(sdk_key, &crate::HashAlgorithm::Sha256),
        );
        let default_options = StatsigOptions::default();
        let options_ref = options.unwrap_or(&default_options);

        StatsigDataStoreSpecsAdapter {
            data_adapter,
            cache_key,
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
                () = sleep(self.sync_interval) => {
                    let update = self.data_adapter.get(&self.cache_key).await;
                    match update {
                        Ok(update) => {
                        match self.listener.try_read_for(std::time::Duration::from_secs(5)) {
                            Some(maybe_listener) => {
                                match maybe_listener.as_ref() {
                                    Some(listener) => {
                                        match listener.did_receive_specs_update(SpecsUpdate {
                                            data: ResponseData::from_bytes(update.result.unwrap_or_default().into_bytes()),
                                            source: SpecsSource::Adapter("DataStore".to_string()),
                                            received_at: Utc::now().timestamp_millis() as u64,
                                            source_api: None,
                                    }) {
                                        Ok(()) => {},
                                        Err(_) => log_w!(TAG, "DataStoreAdapter - Failed to capture"),
                                    }},
                                    _ => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                                }
                            }
                            None => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                            }
                        },
                        Err(_) => log_w!(TAG, "DataAdapterSpecAdatper - Failed to capture"),
                    }
                }
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
            Some(data) => match &self
                .listener
                .try_read_for(std::time::Duration::from_secs(5))
            {
                Some(read_lock) => match read_lock.as_ref() {
                    Some(listener) => {
                        listener.did_receive_specs_update(SpecsUpdate {
                            data: ResponseData::from_bytes(data.into_bytes()),
                            source: SpecsSource::Adapter("DataStore".to_string()),
                            received_at: Utc::now().timestamp_millis() as u64,
                            source_api: None,
                        })?;
                        Ok(())
                    }
                    None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
                },
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            None => Err(StatsigErr::DataStoreFailure("Empty result".to_string())),
        }
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self
            .listener
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut lock) => *lock = Some(listener),
            None => {
                log_e!(TAG, "Failed to acquire write lock on listener");
            }
        }
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // Support polling updates function should be pretty cheap. But we have to make it async
        let should_schedule = self
            .data_adapter
            .support_polling_updates_for(RequestPath::RulesetsV2)
            .await;

        if !should_schedule {
            return Err(StatsigErr::SpecsAdapterSkipPoll(self.get_type_name()));
        }

        let weak_self = Arc::downgrade(&self);

        statsig_runtime.spawn(
            "data_adapter_spec_adapter",
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
        time::timeout(timeout, async { self.data_adapter.shutdown().await })
            .await
            .map_err(|e| StatsigErr::DataStoreFailure(format!("Failed to shutdown: {e}")))?
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigDataStoreSpecAdapter).to_string()
    }
}
