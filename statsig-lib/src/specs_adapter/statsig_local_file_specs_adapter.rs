use crate::specs_adapter::{SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use async_trait::async_trait;
use chrono::Utc;

use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::runtime::Handle;

pub struct StatsigLocalFileSpecsAdapter {
    file_path: String,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
}

impl StatsigLocalFileSpecsAdapter {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            listener: RwLock::new(None),
        }
    }

    pub fn resync_from_file(&self) -> Result<(), StatsigErr> {
        let data = match std::fs::read_to_string(&self.file_path) {
            Ok(data) => data,
            Err(e) => return Err(StatsigErr::SpecsLocalFileReadFailure(e.to_string())),
        };

        match &self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => {
                    listener.did_receive_specs_update(SpecsUpdate {
                        data,
                        source: SpecsSource::Bootstrap,
                        received_at: Utc::now().timestamp_millis() as u64,
                    });
                    Ok(())
                }
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => return Err(StatsigErr::LockFailure(e.to_string())),
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigLocalFileSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr> {
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
        }

        self.resync_from_file()
    }

    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn schedule_background_sync(
        self: Arc<Self>,
        _runtime_handle: &Handle,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(StatsigLocalFileSpecsAdapter).to_string()
    }
}
