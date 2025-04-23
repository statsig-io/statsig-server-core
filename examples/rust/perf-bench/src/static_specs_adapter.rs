use async_trait::async_trait;
use statsig_rust::{
    SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct StaticSpecsAdapter {
    json_data_path: String,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
}

impl StaticSpecsAdapter {
    pub fn with_data(path: &str) -> Self {
        Self {
            json_data_path: path.to_string(),
            listener: RwLock::new(None),
        }
    }

    pub async fn manually_sync_specs(&self, _current_lcut: Option<u64>) -> Result<(), StatsigErr> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(self.json_data_path.as_str());

        let data = fs::read_to_string(path).expect("Unable to read file");

        let update = SpecsUpdate {
            data,
            source: SpecsSource::Bootstrap,
            received_at: 1,
        };

        self.listener
            .write()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .did_receive_specs_update(update)?;

        Ok(())
    }
}

#[async_trait]
impl SpecsAdapter for StaticSpecsAdapter {
    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
        }
    }

    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        let lcut = match self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.get_current_specs_info().lcut,
                None => None,
            },
            Err(_) => None,
        };
        self.manually_sync_specs(lcut).await
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(MockSpecsAdapter).to_string()
    }
}
