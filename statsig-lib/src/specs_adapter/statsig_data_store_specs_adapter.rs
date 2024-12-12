use std::sync::RwLock;
use std::{sync::Arc, time::Duration};
use tokio::time::{self, sleep};

use super::statsig_http_specs_adapter::DEFAULT_SYNC_INTERVAL_MS;
use super::{SpecsSource, SpecsUpdate};
use crate::data_store_interface::{
    get_data_adapter_key, CompressFormat, DataStoreTrait, RequestPath,
};
use crate::hashing::Hashing;
use crate::{log_e, log_w, SpecsAdapter, SpecsUpdateListener};
use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;

const TAG: &str = stringify!(StatsigDatStoreSpecAdapter);

pub struct StatsigDataStoreSpecsAdapter {
    data_adapter: Arc<dyn DataStoreTrait>,
    cache_key: String,
    sync_interval: Duration,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
}

impl StatsigDataStoreSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        data_adapter: Arc<dyn DataStoreTrait>,
        hashing: &Hashing,
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
        }
    }

    fn set_listener(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_e!(
                    TAG,
                    "StatsiDataAdapterSpecsAdapter - Failed to acquire write lock on listener: {}",
                    e
                );
            }
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigDataStoreSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        self.set_listener(listener.clone());
        self.data_adapter.initialize().await?;
        let update = self.data_adapter.get(&self.cache_key).await?;
        match update.result {
            Some(data) => listener.did_receive_specs_update(SpecsUpdate {
                data: data,
                source: SpecsSource::DataAdapter,
                received_at: Utc::now().timestamp_millis() as u64,
            })?,
            None => return Err(StatsigErr::DataStoreFailure("Empty result".to_string())),
        }
        Ok(())
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        // Support polling updates function should be pretty cheap. But we have to make it async
        let should_schedule = tokio::task::block_in_place(|| {
            statsig_runtime.get_handle().block_on(async {self.data_adapter.support_polling_updates_for(RequestPath::RulesetsV2).await})
        });
        if !should_schedule {
            return Err(StatsigErr::DataStoreSkipPoll);
        } else  {
            statsig_runtime.spawn(
                "data_adapter_spec_adapter",
                move |shutdown_notify| async move {
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
                                                        source: SpecsSource::DataAdapter,
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
                                _ = shutdown_notify.notified() => {
                                    break;
                                }
                            }
                        }
                    
                },
            );
        }
        

        Ok(())
    }

    async fn shutdown(
        &self,
        timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        time::timeout(timeout, async { self.data_adapter.shutdown().await })
            .await
            .map_err(|e| {
                StatsigErr::DataStoreFailure(format!("Failed to shutdown: {}", e.to_string()))
            })?
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigDatStoreSpecAdapter).to_string()
    }
}
