use serde_json::Value;
use statsig_rust::Statsig;
use std::time::{Duration, Instant};

use super::mock_event_logging_adapter::MockEventLoggingAdapter;

const DIAGNOSTICS_EVENT: &str = "statsig::diagnostics";

/// Every `statsig::diagnostics` event logged so far, as (context, markers).
pub fn get_diagnostics_events(adapter: &MockEventLoggingAdapter) -> Vec<(String, Vec<Value>)> {
    let payloads = adapter.logged_payloads.lock().unwrap();
    payloads
        .iter()
        .filter_map(|payload| payload.events.as_array())
        .flatten()
        .filter(|event| event["eventName"] == DIAGNOSTICS_EVENT)
        .map(|event| {
            let metadata = &event["metadata"];
            let context = metadata["context"].as_str().unwrap_or_default().to_string();
            let markers = metadata["markers"]
                .as_str()
                .and_then(|m| serde_json::from_str::<Vec<Value>>(m).ok())
                .unwrap_or_default();
            (context, markers)
        })
        .collect()
}

/// Markers from every logged diagnostics event with the given context, in log order.
pub fn get_markers_for_context(adapter: &MockEventLoggingAdapter, context: &str) -> Vec<Value> {
    get_diagnostics_events(adapter)
        .into_iter()
        .filter(|(c, _)| c == context)
        .flat_map(|(_, markers)| markers)
        .collect()
}

/// Flushes until a diagnostics event with the given context is logged.
/// Diagnostics reach the event queue asynchronously, so one flush is not enough.
pub async fn flush_until_context_logged(
    statsig: &Statsig,
    adapter: &MockEventLoggingAdapter,
    context: &str,
) -> Vec<Value> {
    let start = Instant::now();
    loop {
        statsig.flush_events().await;
        let markers = get_markers_for_context(adapter, context);
        if !markers.is_empty() {
            return markers;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("No '{context}' diagnostics event was logged");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// "key:step:action" for each marker, e.g. "download_config_specs:network_request:start".
pub fn summarize_markers(markers: &[Value]) -> Vec<String> {
    markers
        .iter()
        .map(|m| {
            let mut parts = vec![m["key"].as_str().unwrap_or_default()];
            if let Some(step) = m["step"].as_str() {
                parts.push(step);
            }
            parts.push(m["action"].as_str().unwrap_or_default());
            parts.join(":")
        })
        .collect()
}

/// Markers whose key is one of `keys`.
pub fn markers_with_keys<'a>(markers: &'a [Value], keys: &[&str]) -> Vec<&'a Value> {
    markers
        .iter()
        .filter(|m| keys.contains(&m["key"].as_str().unwrap_or_default()))
        .collect()
}
