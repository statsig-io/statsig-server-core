use crate::hashing::djb2;
use crate::networking::ResponseData;
use crate::specs_adapter::statsig_http_specs_adapter::SpecsSyncTrigger;
use crate::specs_adapter::{SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener};
use crate::specs_response::spec_types::SpecsResponseFull;
use crate::statsig_err::StatsigErr;
use crate::{log_e, log_w, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

use super::{SpecsInfo, StatsigHttpSpecsAdapter};

const TAG: &str = stringify!(StatsigLocalFileSpecsAdapter);

pub struct StatsigLocalFileSpecsAdapter {
    file_path: String,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
    http_adapter: StatsigHttpSpecsAdapter,
}

impl StatsigLocalFileSpecsAdapter {
    #[must_use]
    pub fn new(
        sdk_key: &str,
        output_directory: &str,
        specs_url: Option<String>,
        fallback_to_statsig_api: bool,
        disable_network: bool,
    ) -> Self {
        let hashed_key = djb2(sdk_key);
        let file_path = format!("{output_directory}/{hashed_key}_specs.json");

        let options = StatsigOptions {
            specs_url,
            disable_network: Some(disable_network),
            fallback_to_statsig_api: Some(fallback_to_statsig_api),
            ..Default::default()
        };

        Self {
            file_path,
            listener: RwLock::new(None),
            http_adapter: StatsigHttpSpecsAdapter::new(sdk_key, Some(&options), None),
        }
    }

    pub async fn fetch_and_write_to_file(&self) -> Result<(), StatsigErr> {
        let specs_info = match self.read_specs_from_file() {
            Ok(Some(specs)) => SpecsInfo {
                lcut: Some(specs.time),
                checksum: specs.checksum,
                source: SpecsSource::Adapter("FileBased".to_owned()),
                source_api: None,
            },
            _ => SpecsInfo::empty(),
        };

        let data = match self
            .http_adapter
            .fetch_specs_from_network(specs_info, SpecsSyncTrigger::Manual)
            .await
        {
            Ok(mut response) => match response.data.read_to_string() {
                Ok(data) => data,
                Err(e) => {
                    return Err(StatsigErr::SerializationError(e.to_string()));
                }
            },
            Err(e) => {
                return Err(StatsigErr::NetworkError(e));
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
        let data = match std::fs::read(&self.file_path) {
            Ok(data) => ResponseData::from_bytes(data),
            Err(e) => {
                return Err(StatsigErr::FileError(e.to_string()));
            }
        };

        match &self
            .listener
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(lock) => match lock.as_ref() {
                Some(listener) => listener.did_receive_specs_update(SpecsUpdate {
                    data,
                    source: SpecsSource::Adapter("FileBased".to_owned()),
                    received_at: Utc::now().timestamp_millis() as u64,
                    source_api: None,
                }),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            None => Err(StatsigErr::LockFailure(
                "Failed to acquire read lock on listener".to_string(),
            )),
        }
    }

    fn read_specs_from_file(&self) -> Result<Option<SpecsResponseFull>, StatsigErr> {
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

    fn parse_specs_data_to_full_response(&self, data: &str) -> Option<SpecsResponseFull> {
        match serde_json::from_slice::<SpecsResponseFull>(data.as_bytes()) {
            Ok(response) => Some(response),
            Err(e) => {
                log_w!(TAG, "Failed to parse specs data: {}", e);
                None
            }
        }
    }

    fn write_specs_to_file(&self, data: &str) {
        match std::fs::write(&self.file_path, data) {
            Ok(()) => (),
            Err(e) => log_w!(TAG, "Failed to write specs to file: {}", e),
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigLocalFileSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.resync_from_file()
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

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigLocalFileSpecsAdapter).to_string()
    }
}
