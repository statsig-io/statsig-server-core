use crate::specs_adapter::{SpecsAdapter, SpecsSource, SpecsUpdate, SpecsUpdateListener};
use crate::statsig_err::StatsigErr;
use crate::{log_e, StatsigRuntime};
use async_trait::async_trait;
use chrono::Utc;

use std::sync::{Arc, RwLock};
use std::time::Duration;
pub struct StatsigBootstrapSpecsAdapter {
    data: RwLock<String>,
    listener: RwLock<Option<Arc<dyn SpecsUpdateListener>>>,
}
const TAG: &str = stringify!(StatsigBootstrapSpecsAdapter);

impl StatsigBootstrapSpecsAdapter {
    #[must_use]
    pub fn new(data: String) -> Self {
        Self {
            data: RwLock::new(data),
            listener: RwLock::new(None),
        }
    }

    pub fn set_data(&self, data: String) -> Result<(), StatsigErr> {
        match self.data.write() {
            Ok(mut lock) => *lock = data.clone(),
            Err(e) => return Err(StatsigErr::LockFailure(e.to_string())),
        };

        self.push_update()
    }

    fn push_update(&self) -> Result<(), StatsigErr> {
        let data = match self.data.read() {
            Ok(lock) => lock.clone(),
            Err(e) => return Err(StatsigErr::LockFailure(e.to_string())),
        };

        match &self.listener.read() {
            Ok(lock) => match lock.as_ref() {
                Some(listener) => listener.did_receive_specs_update(SpecsUpdate {
                    data: data.into_bytes(),
                    source: SpecsSource::Bootstrap,
                    received_at: Utc::now().timestamp_millis() as u64,
                    source_api: None,
                }),
                None => Err(StatsigErr::UnstartedAdapter("Listener not set".to_string())),
            },
            Err(e) => Err(StatsigErr::LockFailure(e.to_string())),
        }
    }
}

#[async_trait]
impl SpecsAdapter for StatsigBootstrapSpecsAdapter {
    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.push_update()
    }

    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        match self.listener.write() {
            Ok(mut lock) => *lock = Some(listener),
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on listener: {}", e);
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
        stringify!(StatsigBootstrapSpecsAdapter).to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::SpecsInfo;

    use super::*;
    use std::sync::Arc;

    struct TestListener {
        received_update: RwLock<Option<SpecsUpdate>>,
    }

    impl TestListener {
        fn new() -> Self {
            Self {
                received_update: RwLock::new(None),
            }
        }
    }

    #[async_trait]
    impl SpecsUpdateListener for TestListener {
        fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr> {
            if let Ok(mut lock) = self.received_update.write() {
                *lock = Some(update);
            }
            Ok(())
        }

        fn get_current_specs_info(&self) -> SpecsInfo {
            SpecsInfo::empty()
        }
    }

    #[tokio::test]
    async fn test_manually_sync_specs() {
        let test_data = serde_json::json!({
            "feature_gates": {},
            "dynamic_configs": {},
            "layer_configs": {},
        })
        .to_string();

        let adapter = Arc::new(StatsigBootstrapSpecsAdapter::new(test_data.clone()));
        let listener = Arc::new(TestListener::new());

        let statsig_rt = StatsigRuntime::get_runtime();
        adapter.initialize(listener.clone());
        adapter.clone().start(&statsig_rt).await.unwrap();

        if let Ok(lock) = listener.clone().received_update.read() {
            let update = lock.as_ref().unwrap();
            assert_eq!(update.source, SpecsSource::Bootstrap);
            assert_eq!(update.data, test_data.into_bytes());
        }
    }

    #[tokio::test]
    async fn test_set_data() {
        let statsig_rt = StatsigRuntime::get_runtime();

        let adapter = Arc::new(StatsigBootstrapSpecsAdapter::new(String::new()));

        let listener = Arc::new(TestListener::new());
        adapter.initialize(listener.clone());
        adapter.clone().start(&statsig_rt).await.unwrap();

        let test_data = "{\"some\": \"value\"}".to_string();
        let result = adapter.set_data(test_data.clone());
        assert!(result.is_ok());

        if let Ok(lock) = listener.clone().received_update.read() {
            let update = lock.as_ref().unwrap();
            assert_eq!(update.source, SpecsSource::Bootstrap);
            assert_eq!(update.data, test_data.into_bytes());
        }
    }
}
