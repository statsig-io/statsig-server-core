use std::sync::Arc;

use crate::{StatsigErr, StatsigRuntime};
use crate::{event_logging_adapter::EventLoggingAdapter};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::LogEventRequest;

const MAX_PENDING_REQUESTS: usize = 10;

#[derive(Serialize, Deserialize)]
pub struct PendingLogRequests {
    requests: Vec<LogEventRequest>,
}

pub struct StatsigLocalFileEventLoggingAdapter {
    file_path: String,
}

impl StatsigLocalFileEventLoggingAdapter {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }

    pub fn get_current_requests(&self) -> Result<PendingLogRequests, StatsigErr> {
        let file_contents = std::fs::read_to_string(&self.file_path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        Ok(serde_json::from_str(&file_contents).map_err(|e| {
            StatsigErr::JsonParseError(stringify!(PendingLogRequests).to_string(), e.to_string())
        })?)
    }

    pub fn get_and_clear_current_requests(&self) -> Result<PendingLogRequests, StatsigErr> {
        let result = self.get_current_requests()?;
        std::fs::remove_file(&self.file_path).map_err(|e| StatsigErr::FileError(e.to_string()))?;
        Ok(result)
    }
}

#[async_trait]
impl EventLoggingAdapter for StatsigLocalFileEventLoggingAdapter {
    async fn start(
        &self,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let mut current_requests = self.get_current_requests()?;

        current_requests.requests.push(request);

        while current_requests.requests.len() >= MAX_PENDING_REQUESTS {
            current_requests.requests.remove(0);
        }

        let json = serde_json::to_string(&current_requests).map_err(|e| {
            StatsigErr::JsonParseError(stringify!(PendingLogRequests).to_string(), e.to_string())
        })?;

        std::fs::write(&self.file_path, json).map_err(|e| StatsigErr::FileError(e.to_string()))?;

        Ok(true)
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }
}
