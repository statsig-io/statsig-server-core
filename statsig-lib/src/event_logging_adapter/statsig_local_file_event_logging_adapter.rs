use std::collections::HashSet;
use std::io::Write;
use std::sync::Arc;

use super::log_event_payload::LogEventRequest;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::hashing::djb2;
use crate::log_event_payload::LogEventPayload;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_e, StatsigErr, StatsigHttpEventLoggingAdapter, StatsigRuntime};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

const TAG: &str = stringify!(StatsigLocalFileEventLoggingAdapter);
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
        let current_requests = match self.get_and_clear_current_requests()? {
            Some(requests) => requests,
            None => return Ok(()),
        };

        let mut seen_exposures = HashSet::new();
        let mut processed_events = vec![];

        for line in current_requests.lines() {
            let events: Vec<StatsigEventInternal> = match serde_json::from_str(line) {
                Ok(events) => events,
                Err(e) => {
                    log_e!(TAG, "Failed to parse events in file: {}", e.to_string());
                    continue
                }
            };

            for event in events {
                if !event.is_exposure_event() {
                    processed_events.push(event);
                    continue;
                }

                let key = create_merge_key(&event);
                if seen_exposures.contains(&key) {
                    continue;
                }

                seen_exposures.insert(key);
                processed_events.push(event);
            }
        }

        let chunks = processed_events.chunks(1000);
        let tasks = chunks.map(|chunk| async move {
            let request = LogEventRequest {
                event_count: chunk.len() as u64,
                payload: LogEventPayload {
                    events: json!(chunk),
                    statsig_metadata: StatsigMetadata::get_as_json(),
                },
            };

            let result = self.http_adapter.send_events_over_http(&request).await;
            (request, result)
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

    fn get_and_clear_current_requests(&self) -> Result<Option<String>, StatsigErr> {
        if !std::path::Path::new(&self.file_path).exists() {
            return Ok(None);
        }

        let file_contents = std::fs::read_to_string(&self.file_path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        std::fs::remove_file(&self.file_path).map_err(|e| StatsigErr::FileError(e.to_string()))?;
        Ok(Some(file_contents))
    }
}

fn create_merge_key(event: &StatsigEventInternal) -> String {
    let mut metadata_string = String::new();
    if let Some(metadata) = &event.event_data.metadata {
        let mut entries: Vec<(String, String)> = metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        metadata_string = entries
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<String>>()
            .join(",");
    }

    format!(
        "{}|{}|{}",
        event.event_data.event_name, event.user.value, metadata_string
    )
}

#[async_trait]
impl EventLoggingAdapter for StatsigLocalFileEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let json = request.payload.events.to_string();

        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?
            .write_all(format!("{}\n", json).as_bytes())
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        Ok(true)
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        false
    }
}
