use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use tokio::runtime::{Handle};
use statsig::{SpecsAdapter, SpecsUpdateListener, SpecsUpdate, SpecsSource, StatsigErr};

pub struct MockSpecsAdapter {
    json_data_path: String,
    should_throw: bool,
    delay_ms: Option<u64>,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
}

impl MockSpecsAdapter {
    pub fn with_data(path: &str) -> Self {
        Self {
            json_data_path: path.to_string(),
            should_throw: false,
            delay_ms: None,
            listener: RwLock::new(None)
        }
    }

    pub fn throwing() -> Self {
        Self {
            json_data_path: "".to_string(),
            should_throw: true,
            delay_ms: None,
            listener: RwLock::new(None)
        }
    }

    pub fn delayed(path: &str, delay_ms: u64) -> Self {
        Self {
            json_data_path: path.to_string(),
            should_throw: false,
            delay_ms: Some(delay_ms),
            listener: RwLock::new(None)
        }
    }
}

#[async_trait]
impl SpecsAdapter for MockSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>
    ) -> Result<(), StatsigErr> {
        if let Ok(mut mut_listener) = self.listener.write() {
            *mut_listener = Some(listener);
        }

        Ok(())
    }

    async fn manually_sync_specs(&self, _current_lcut: Option<u64>) -> Result<(), StatsigErr> {
        if self.should_throw {
            return Err(StatsigErr::CustomError("Not today".to_string()));
        }

        if let Some(delay) = self.delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(self.json_data_path.as_str());

        let data = fs::read_to_string(path).expect("Unable to read file");
        let update = SpecsUpdate {
            data,
            source: SpecsSource::Bootstrap,
            received_at: Utc::now().timestamp_millis() as u64,
        };

        self.listener.write().as_ref().unwrap().as_ref().unwrap().did_receive_specs_update(update);
        Ok(())
    }

    async fn shutdown(&self, _timeout: Duration) -> Result<(), StatsigErr> {
        Ok(())
    }
}
