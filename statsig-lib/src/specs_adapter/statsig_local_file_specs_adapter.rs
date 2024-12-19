use crate::hashing::djb2;
use crate::spec_types::{SpecsResponse, SpecsResponseFull};
use crate::specs_adapter::{SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::{log_w, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use super::StatsigHttpSpecsAdapter;

const TAG: &str = stringify!(StatsigLocalFileSpecsAdapter);

pub struct StatsigLocalFileSpecsAdapter {
    file_path: String,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    http_adapter: StatsigHttpSpecsAdapter,
}

impl StatsigLocalFileSpecsAdapter {
    pub fn new(
        sdk_key: &str,
        output_directory: &str,
        specs_url: Option<String>,
        fallback_to_statsig_api: bool,
    ) -> Self {
        let hashed_key = djb2(sdk_key);
        let file_path = format!("{}/{}_specs.json", output_directory, hashed_key);

        Self {
            file_path,
            listener: RwLock::new(None),
            http_adapter: StatsigHttpSpecsAdapter::new(
                sdk_key,
                specs_url.as_ref(),
                fallback_to_statsig_api,
                None,
            ),
        }
    }

    pub async fn fetch_and_write_to_file(&self) -> Result<(), StatsigErr> {
        let lcut = match self.read_specs_from_file() {
            Ok(Some(specs)) => Some(specs.time),
            _ => None,
        };

        let data = match self.http_adapter.fetch_specs_from_network(lcut).await {
            Some(data) => data,
            None => {
                return Err(StatsigErr::NetworkError("No data received".to_string()));
            }
        };

        if let Some(response) = self.parse_specs_data_to_full_response(&data) {
            if response.has_updates {
                self.write_specs_to_file(&data);
            }
        }

        Ok(())
    }

    pub fn resync_from_file(&self) -> Result<(), StatsigErr> {
        let data = match std::fs::read_to_string(&self.file_path) {
            Ok(data) => data,
            Err(e) => {
                return Err(StatsigErr::FileError(e.to_string()));
            }
        };

        match &self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.did_receive_specs_update(SpecsUpdate {
                    data,
                    source: SpecsSource::Bootstrap,
                    received_at: Utc::now().timestamp_millis() as u64,
                }),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => Err(StatsigErr::LockFailure(e.to_string())),
        }
    }

    fn read_specs_from_file(&self) -> Result<Option<Box<SpecsResponseFull>>, StatsigErr> {
        if !std::path::Path::new(&self.file_path).exists() {
            return Ok(None);
        }

        let data = match std::fs::read_to_string(&self.file_path) {
            Ok(data) => data,
            Err(e) => {
                return Err(StatsigErr::FileError(e.to_string()));
            }
        };

        Ok(self.parse_specs_data_to_full_response(&data))
    }

    fn parse_specs_data_to_full_response(&self, data: &str) -> Option<Box<SpecsResponseFull>> {
        match serde_json::from_str::<SpecsResponse>(data) {
            Ok(SpecsResponse::Full(full)) => Some(full),
            Ok(SpecsResponse::NoUpdates(_)) => None,
            Err(e) => {
                log_w!(TAG, "Failed to parse specs data: {}", e);
                None
            }
        }
    }

    fn write_specs_to_file(&self, data: &str) {
        match std::fs::write(&self.file_path, data) {
            Ok(_) => (),
            Err(e) => log_w!(TAG, "Failed to write specs to file: {}", e),
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigLocalFileSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
        }

        self.resync_from_file()
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigLocalFileSpecsAdapter).to_string()
    }
}
