use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::Arc;

use super::log_event_payload::LogEventRequest;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::hashing::djb2;
use crate::log_event_payload::LogEventPayload;
use crate::statsig_metadata::StatsigMetadata;
use crate::{
    log_d, log_e, StatsigErr, StatsigHttpEventLoggingAdapter, StatsigOptions, StatsigRuntime,
};
use async_trait::async_trait;
use file_guard::Lock;
use rand::Rng;
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
    #[must_use]
    pub fn new(
        sdk_key: &str,
        output_directory: &str,
        log_event_url: Option<String>,
        disable_network: bool,
    ) -> Self {
        let hashed_key = djb2(sdk_key);
        let file_path = format!("{output_directory}/{hashed_key}_events.json");

        let options = StatsigOptions {
            log_event_url,
            disable_network: Some(disable_network),
            ..Default::default()
        };

        Self {
            file_path,
            http_adapter: StatsigHttpEventLoggingAdapter::new(sdk_key, Some(&options)),
        }
    }

    pub async fn send_pending_events(&self) -> Result<(), StatsigErr> {
        log_d!(TAG, "Sending pending events...");
        let current_requests = if let Some(requests) = read_and_clear_file(&self.file_path)? {
            requests
        } else {
            log_d!(TAG, "No events found");
            return Ok(());
        };

        let processed_events = process_events(&current_requests);
        let chunks = processed_events.chunks(1000);
        let tasks = chunks.map(|chunk| async move {
            let request = LogEventRequest {
                event_count: chunk.len() as u64,
                payload: LogEventPayload {
                    events: json!(chunk),
                    statsig_metadata: StatsigMetadata::get_as_json(),
                },
                retries: 0,
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

        log_d!(TAG, "All events sent");
        Ok(())
    }
}

fn read_and_clear_file(file_path: &str) -> Result<Option<String>, StatsigErr> {
    log_d!(TAG, "Retrieving pending events from {}", file_path);

    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(None);
    }

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_path)
        .map_err(|e| StatsigErr::FileError(e.to_string()))?;

    let mut lock = file_guard::lock(&mut file, Lock::Exclusive, 0, 1)
        .map_err(|e| StatsigErr::FileError(e.to_string()))?;

    let mut file_contents = String::new();
    (*lock)
        .read_to_string(&mut file_contents)
        .map_err(|e| StatsigErr::FileError(e.to_string()))?;

    // Truncate the file to clear its contents
    (*lock)
        .set_len(0)
        .map_err(|e| StatsigErr::FileError(e.to_string()))?;

    Ok(Some(file_contents))
}

fn process_events(current_requests: &str) -> Vec<StatsigEventInternal> {
    let mut seen_exposures = HashSet::new();
    let mut processed_events = vec![];

    for line in current_requests.lines() {
        let events: Vec<StatsigEventInternal> = match serde_json::from_str(line) {
            Ok(events) => events,
            Err(e) => {
                log_e!(TAG, "Failed to parse events in file: {}", e.to_string());
                continue;
            }
        };

        for event in events {
            if event.is_diagnostic_event() && !should_sample_sdk_diagnostics() {
                continue;
            }

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

    processed_events
}

fn create_merge_key(event: &StatsigEventInternal) -> String {
    let mut metadata_parts = Vec::new();
    if let Some(metadata) = &event.event_data.metadata {
        if let Some(name) = metadata.get("gate") {
            metadata_parts.push(format!("g.{name}|"));
        }

        if let Some(name) = metadata.get("config") {
            metadata_parts.push(format!("c.{name}|"));
        }

        if let Some(name) = metadata.get("parameterName") {
            metadata_parts.push(format!("pn.{name}|"));
        }

        if let Some(name) = metadata.get("allocatedExperiment") {
            metadata_parts.push(format!("ae.{name}|"));
        }

        if let Some(rule_id) = metadata.get("ruleID") {
            metadata_parts.push(format!("r.{rule_id}|"));
        }
    }

    format!(
        "{}|{}|{}",
        event.event_data.event_name,
        event.user.data.create_user_values_hash(),
        metadata_parts.concat()
    )
}

// PHP initializes per request, so we get a diagnostics event per request.
// This samples quite aggressively to compensate for that
fn should_sample_sdk_diagnostics() -> bool {
    let random_number = rand::thread_rng().gen_range(0..10000);
    random_number < 1
}

#[async_trait]
impl EventLoggingAdapter for StatsigLocalFileEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let json = request.payload.events.to_string();

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        let mut lock = file_guard::lock(&mut file, Lock::Exclusive, 0, 1)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        (*lock)
            .write_all(format!("{json}\n").as_bytes())
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
