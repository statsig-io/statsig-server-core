use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;

use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::OpsStatsForInstance;
use crate::utils::get_api_from_url;

const CONFIG_SYNC_OVERALL_LATENCY_METRIC: &str = "config_sync_overall.latency";
const CONFIG_SYNC_OVERALL_FORMAT_TAG: &str = "format";
const CONFIG_SYNC_OVERALL_SOURCE_API_TAG: &str = "source_api";
const CONFIG_SYNC_OVERALL_ERROR_TAG: &str = "error";
const CONFIG_SYNC_OVERALL_NETWORK_SUCCESS_TAG: &str = "network_success";
const CONFIG_SYNC_OVERALL_PROCESS_SUCCESS_TAG: &str = "process_success";
const CONFIG_SYNC_OVERALL_DELTAS_USED_TAG: &str = "deltas_used";

#[allow(clippy::too_many_arguments)]
pub fn log_config_sync_overall_latency(
    ops_stats: &Arc<OpsStatsForInstance>,
    sync_start_ms: u64,
    source_api: &str,
    response_format: &str,
    network_success: bool,
    process_success: bool,
    error: String,
    deltas_used: bool,
) {
    let latency_ms = (Utc::now().timestamp_millis() as u64).saturating_sub(sync_start_ms) as f64;
    ops_stats.log(ObservabilityEvent::new_event(
        MetricType::Dist,
        CONFIG_SYNC_OVERALL_LATENCY_METRIC.to_string(),
        latency_ms,
        Some(HashMap::from([
            (
                CONFIG_SYNC_OVERALL_SOURCE_API_TAG.to_string(),
                if source_api == "datastore" {
                    source_api.to_string()
                } else {
                    get_api_from_url(source_api)
                },
            ),
            (
                CONFIG_SYNC_OVERALL_FORMAT_TAG.to_string(),
                response_format.to_string(),
            ),
            (CONFIG_SYNC_OVERALL_ERROR_TAG.to_string(), error),
            (
                CONFIG_SYNC_OVERALL_NETWORK_SUCCESS_TAG.to_string(),
                network_success.to_string(),
            ),
            (
                CONFIG_SYNC_OVERALL_PROCESS_SUCCESS_TAG.to_string(),
                process_success.to_string(),
            ),
            (
                CONFIG_SYNC_OVERALL_DELTAS_USED_TAG.to_string(),
                deltas_used.to_string(),
            ),
        ])),
    ));
}
