use std::sync::Arc;

use super::log_event_payload::LogEventRequest;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::hashing::djb2;
use crate::{StatsigErr, StatsigHttpEventLoggingAdapter, StatsigRuntime};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
const MAX_PENDING_REQUESTS: usize = 10;

#[derive(Serialize, Deserialize)]
pub struct PendingLogRequests {
    requests: Vec<LogEventRequest>,
}

pub struct StatsigLocalFileEventLoggingAdapter {
    file_path: String,
    http_adapter: StatsigHttpEventLoggingAdapter,
}

impl StatsigLocalFileEventLoggingAdapter {
    pub fn new(sdk_key: &str, output_directory: &str, log_event_url: Option<String>) -> Self {
        let hashed_key = djb2(sdk_key);
        let file_path = format!("{}/{}_events.json", output_directory, hashed_key);

        Self {
            file_path,
            http_adapter: StatsigHttpEventLoggingAdapter::new(sdk_key, log_event_url.as_ref()),
        }
    }

    pub async fn send_pending_events(&self) -> Result<(), StatsigErr> {
        let current_requests = self.get_and_clear_current_requests()?;

        if current_requests.requests.is_empty() {
            return Ok(());
        }

        let tasks = current_requests.requests.into_iter().map(|req| async {
            let result = self.http_adapter.send_events_over_http(&req).await;
            (req, result)
        });

        let results = futures::future::join_all(tasks).await;
        let mut failed_requests = Vec::new();

        for (request, result) in results {
            match result {
                Ok(true) => (),
                _ => failed_requests.push(request),
            }
        }

        if !failed_requests.is_empty() {
            self.log_events(failed_requests.remove(0)).await?;
        }

        Ok(())
    }

    fn get_current_requests(&self) -> Result<PendingLogRequests, StatsigErr> {
        if !std::path::Path::new(&self.file_path).exists() {
            return Ok(PendingLogRequests { requests: vec![] });
        }

        let file_contents = std::fs::read_to_string(&self.file_path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        Ok(serde_json::from_str(&file_contents).map_err(|e| {
            StatsigErr::JsonParseError(stringify!(PendingLogRequests).to_string(), e.to_string())
        })?)
    }

    fn get_and_clear_current_requests(&self) -> Result<PendingLogRequests, StatsigErr> {
        let result = self.get_current_requests()?;
        std::fs::remove_file(&self.file_path).map_err(|e| StatsigErr::FileError(e.to_string()))?;
        Ok(result)
    }
}

#[async_trait]
impl EventLoggingAdapter for StatsigLocalFileEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let mut current_requests = self.get_current_requests()?;

        if !attempt_to_append_events(&mut current_requests, &request) {
            current_requests.requests.push(request);
        }

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

    fn should_schedule_background_flush(&self) -> bool {
        false
    }
}

fn attempt_to_append_events(current: &mut PendingLogRequests, request: &LogEventRequest) -> bool {
    let last = match current.requests.last_mut() {
        Some(last) => last,
        None => {
            return false;
        }
    };

    if last.event_count >= 500 {
        false
    } else {
        last.merge(request);
        true
    }
}
