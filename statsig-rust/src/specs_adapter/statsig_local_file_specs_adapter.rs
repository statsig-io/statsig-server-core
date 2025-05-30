use crate::hashing::djb2;
use crate::specs_adapter::{SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener};
use crate::specs_response::spec_types::SpecsResponseFull;
use crate::specs_response::spec_types_encoded::DecodedSpecsResponse;
use crate::statsig_err::StatsigErr;
use crate::{log_w, StatsigOptions, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;

use std::sync::{Arc, RwLock};
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
                zstd_dict_id: None, // For backwards compatibility, we only store uncompressed specs
                source: SpecsSource::Adapter("FileBased".to_owned()),
                source_api: None,
            },
            _ => SpecsInfo::empty(),
        };

        let data = match self.http_adapter.fetch_specs_from_network(specs_info).await {
            Ok(response) => match String::from_utf8(response.data) {
                Ok(data) => data,
                Err(e) => {
                    return Err(StatsigErr::SerializationError(e.to_string()));
                }
            },
            Err(e) => {
                return Err(StatsigErr::NetworkError(
                    e,
                    Some("No data received".to_string()),
                ));
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
                    data: data.into_bytes(),
                    source: SpecsSource::Adapter("FileBased".to_owned()),
                    received_at: Utc::now().timestamp_millis() as u64,
                    source_api: None,
                }),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => Err(StatsigErr::LockFailure(e.to_string())),
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
        let mut place = SpecsResponseFull::default();
        match DecodedSpecsResponse::from_slice(data.as_bytes(), &mut place, None) {
            Ok(_) => Some(place),
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
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
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
