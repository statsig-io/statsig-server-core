use super::api_call_diagnostics::ApiCallDiagnostics;
use super::diagnostics_utils::DiagnosticsUtils;
use super::marker::{KeyType, Marker};

use crate::event_logging::event_queue::queued_passthrough::EnqueuePassthroughOp;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::global_configs::{GlobalConfigs, MAX_SAMPLING_RATE};

use crate::log_w;

use crate::event_logging::event_logger::EventLogger;
use parking_lot::Mutex;
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

const MAX_MARKER_COUNT: usize = 50;
pub const DIAGNOSTICS_EVENT: &str = "statsig::diagnostics";

#[derive(Eq, Hash, PartialEq, Clone, Serialize, Debug, Copy)]
pub enum ContextType {
    Initialize,
    ConfigSync,
    ApiCall,
}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextType::Initialize => write!(f, "initialize"),
            ContextType::ConfigSync => write!(f, "config_sync"),
            ContextType::ApiCall => write!(f, "api_call"),
        }
    }
}

/// Operations that finish and get sampled independently share a context but
/// not a marker list, so one operation's send never takes another's markers.
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
enum MarkerGroup {
    /// Everything in the context is sent as one event (initialize, api_call).
    All,
    ConfigSpecs,
    IdLists,
}

impl MarkerGroup {
    fn new(context: ContextType, key: Option<KeyType>) -> Self {
        if context != ContextType::ConfigSync {
            return MarkerGroup::All;
        }

        match key {
            Some(KeyType::DownloadConfigSpecs) => MarkerGroup::ConfigSpecs,
            Some(KeyType::GetIDList | KeyType::GetIDListSources) => MarkerGroup::IdLists,
            _ => MarkerGroup::All,
        }
    }
}

type MarkerBucket = (ContextType, MarkerGroup);

const TAG: &str = stringify!(Diagnostics);
const DEFAULT_SAMPLING_RATE: f64 = 100.0;

pub struct Diagnostics {
    marker_map: Mutex<HashMap<MarkerBucket, Vec<Marker>>>,
    event_logger: Arc<EventLogger>,
    global_configs: Arc<GlobalConfigs>,
    api_call_diagnostics: Arc<ApiCallDiagnostics>,
}

impl Diagnostics {
    pub fn new(event_logger: Arc<EventLogger>, sdk_key: &str) -> Self {
        Self {
            event_logger,
            marker_map: Mutex::new(HashMap::new()),
            global_configs: GlobalConfigs::get_instance(sdk_key),
            api_call_diagnostics: Arc::new(ApiCallDiagnostics::new()),
        }
    }

    pub fn api_call_diagnostics(&self) -> Arc<ApiCallDiagnostics> {
        self.api_call_diagnostics.clone()
    }

    pub fn add_marker(&self, context: ContextType, marker: Marker) {
        let bucket = (context, MarkerGroup::new(context, Some(marker.key)));

        match self.marker_map.try_lock_for(Duration::from_secs(5)) {
            Some(mut map) => {
                let entry = map.entry(bucket).or_insert_with(Vec::new);
                if entry.len() < MAX_MARKER_COUNT {
                    entry.push(marker);
                }
            }
            None => {
                log_w!(TAG, "Failed to add marker: Failed to lock marker_map");
            }
        }
    }

    /// For api_call, called on every event logger tick: sends the window's
    /// markers only when the window ends, then starts the next window.
    pub fn enqueue_diagnostics_event(&self, context: ContextType, key: Option<KeyType>) {
        if context == ContextType::ApiCall
            && !self
                .api_call_diagnostics
                .try_rotate_window(&self.global_configs)
        {
            return;
        }

        let Some(metadata) = self.take_event_metadata(context, key) else {
            return;
        };

        self.event_logger.enqueue(EnqueuePassthroughOp {
            event: StatsigEventInternal::new_diagnostic_event(metadata),
        });
    }

    /// Sends the current api_call window's markers now, without starting a
    /// new window. Used at shutdown so a sampled window is not lost.
    pub fn send_api_call_markers(&self) {
        let Some(metadata) = self.take_event_metadata(ContextType::ApiCall, None) else {
            return;
        };

        self.event_logger.enqueue(EnqueuePassthroughOp {
            event: StatsigEventInternal::new_diagnostic_event(metadata),
        });
    }

    /// Removes the markers that belong to `key`'s operation in `context` and,
    /// if sampled in, returns them formatted as diagnostics event metadata.
    fn take_event_metadata(
        &self,
        context: ContextType,
        key: Option<KeyType>,
    ) -> Option<HashMap<String, String>> {
        let bucket = (context, MarkerGroup::new(context, key));
        let markers = match self.marker_map.try_lock_for(Duration::from_secs(5)) {
            Some(mut map) => map.remove(&bucket)?,
            None => {
                log_w!(TAG, "Failed to take markers: Failed to lock marker_map");
                return None;
            }
        };

        if markers.is_empty() || !self.should_sample(&context, key) {
            return None;
        }

        match DiagnosticsUtils::format_diagnostics_metadata(&context, &markers) {
            Ok(data) => Some(data),
            Err(err) => {
                log_w!(TAG, "Failed to format diagnostics metadata: {}", err);
                None
            }
        }
    }

    pub fn should_sample(&self, context: &ContextType, key: Option<KeyType>) -> bool {
        fn check_sampling_rate(sampling_rate: Option<&f64>) -> bool {
            let mut rng = rand::thread_rng();
            let rand_value = rng.gen::<f64>() * MAX_SAMPLING_RATE;

            match sampling_rate {
                Some(sampling_rate) => rand_value < *sampling_rate,
                None => rand_value < DEFAULT_SAMPLING_RATE,
            }
        }

        if *context == ContextType::Initialize {
            return self
                .global_configs
                .use_diagnostics_sampling_rate("initialize", check_sampling_rate);
        }

        if *context == ContextType::ApiCall {
            // Sampled when the window started; only sampled windows have markers.
            return true;
        }

        if let Some(key) = key {
            match key {
                KeyType::GetIDList | KeyType::GetIDListSources => {
                    return self
                        .global_configs
                        .use_diagnostics_sampling_rate("get_id_list", check_sampling_rate);
                }
                KeyType::DownloadConfigSpecs => {
                    return self
                        .global_configs
                        .use_diagnostics_sampling_rate("dcs", check_sampling_rate);
                }
                _ => {}
            }
        }

        check_sampling_rate(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_logging_adapter::{EventLoggingAdapter, StatsigHttpEventLoggingAdapter};
    use crate::sdk_diagnostics::marker::{ActionType, StepType};
    use crate::{StatsigOptions, StatsigRuntime};
    use serde_json::Value;

    fn create_diagnostics(sdk_key: &str, sampling_rates: &[(&str, f64)]) -> Diagnostics {
        let options = Arc::new(StatsigOptions::default());
        let adapter: Arc<dyn EventLoggingAdapter> =
            Arc::new(StatsigHttpEventLoggingAdapter::new(sdk_key, None));
        let event_logger =
            EventLogger::new(sdk_key, &options, &adapter, &StatsigRuntime::get_runtime());
        GlobalConfigs::get_instance(sdk_key).set_diagnostics_sampling_rates(
            sampling_rates
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        );
        Diagnostics::new(event_logger, sdk_key)
    }

    fn network_marker(key: KeyType, action: ActionType) -> Marker {
        Marker::new(key, action, Some(StepType::NetworkRequest))
    }

    fn marker_keys(metadata: &HashMap<String, String>) -> Vec<String> {
        let markers: Vec<Value> = serde_json::from_str(&metadata["markers"]).unwrap();
        markers
            .iter()
            .map(|m| m["key"].as_str().unwrap().to_string())
            .collect()
    }

    // Config and ID list polls overlap: each one's send must contain only its own markers.
    #[tokio::test]
    async fn test_overlapping_config_sync_operations_are_sent_separately() {
        let diagnostics = create_diagnostics(
            "secret-diag-overlap",
            &[
                ("dcs", MAX_SAMPLING_RATE),
                ("get_id_list", MAX_SAMPLING_RATE),
            ],
        );
        let sync = ContextType::ConfigSync;

        diagnostics.add_marker(
            sync,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::Start),
        );
        diagnostics.add_marker(
            sync,
            network_marker(KeyType::GetIDListSources, ActionType::Start),
        );
        diagnostics.add_marker(sync, network_marker(KeyType::GetIDList, ActionType::Start));
        diagnostics.add_marker(
            sync,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::End),
        );

        let dcs_event = diagnostics
            .take_event_metadata(sync, Some(KeyType::DownloadConfigSpecs))
            .unwrap();
        assert_eq!(dcs_event["context"], "config_sync");
        assert_eq!(
            marker_keys(&dcs_event),
            vec!["download_config_specs", "download_config_specs"]
        );

        // the ID list poll finishes later and still has all of its markers
        diagnostics.add_marker(
            sync,
            network_marker(KeyType::GetIDListSources, ActionType::End),
        );
        let id_list_event = diagnostics
            .take_event_metadata(sync, Some(KeyType::GetIDListSources))
            .unwrap();
        assert_eq!(id_list_event["context"], "config_sync");
        assert_eq!(
            marker_keys(&id_list_event),
            vec!["get_id_list_sources", "get_id_list", "get_id_list_sources"]
        );

        assert!(diagnostics
            .take_event_metadata(sync, Some(KeyType::DownloadConfigSpecs))
            .is_none());
    }

    // An operation that is sampled out drops only its own markers.
    #[tokio::test]
    async fn test_sampling_out_one_config_sync_operation_keeps_the_other() {
        let diagnostics = create_diagnostics(
            "secret-diag-sampling",
            &[("dcs", MAX_SAMPLING_RATE), ("get_id_list", 0.0)],
        );
        let sync = ContextType::ConfigSync;

        diagnostics.add_marker(
            sync,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::Start),
        );
        diagnostics.add_marker(
            sync,
            network_marker(KeyType::GetIDListSources, ActionType::Start),
        );

        assert!(diagnostics
            .take_event_metadata(sync, Some(KeyType::GetIDListSources))
            .is_none());
        let dcs_event = diagnostics
            .take_event_metadata(sync, Some(KeyType::DownloadConfigSpecs))
            .unwrap();
        assert_eq!(marker_keys(&dcs_event), vec!["download_config_specs"]);
    }

    // Initialize is one event with everything, whatever key it is sent with.
    #[tokio::test]
    async fn test_initialize_markers_are_sent_together() {
        let diagnostics = create_diagnostics(
            "secret-diag-initialize",
            &[("initialize", MAX_SAMPLING_RATE)],
        );
        let init = ContextType::Initialize;

        diagnostics.add_marker(init, Marker::new(KeyType::Overall, ActionType::Start, None));
        diagnostics.add_marker(
            init,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::Start),
        );
        diagnostics.add_marker(
            init,
            network_marker(KeyType::GetIDListSources, ActionType::Start),
        );
        diagnostics.add_marker(
            ContextType::ConfigSync,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::Start),
        );

        let event = diagnostics.take_event_metadata(init, None).unwrap();
        assert_eq!(event["context"], "initialize");
        assert_eq!(
            marker_keys(&event),
            vec!["overall", "download_config_specs", "get_id_list_sources"]
        );
    }

    #[tokio::test]
    async fn test_marker_limit_is_per_operation() {
        let diagnostics = create_diagnostics(
            "secret-diag-limit",
            &[
                ("dcs", MAX_SAMPLING_RATE),
                ("get_id_list", MAX_SAMPLING_RATE),
            ],
        );
        let sync = ContextType::ConfigSync;

        for _ in 0..(MAX_MARKER_COUNT + 10) {
            diagnostics.add_marker(sync, network_marker(KeyType::GetIDList, ActionType::Start));
        }
        diagnostics.add_marker(
            sync,
            network_marker(KeyType::DownloadConfigSpecs, ActionType::Start),
        );

        let id_list_event = diagnostics
            .take_event_metadata(sync, Some(KeyType::GetIDListSources))
            .unwrap();
        assert_eq!(marker_keys(&id_list_event).len(), MAX_MARKER_COUNT);
        let dcs_event = diagnostics
            .take_event_metadata(sync, Some(KeyType::DownloadConfigSpecs))
            .unwrap();
        assert_eq!(marker_keys(&dcs_event), vec!["download_config_specs"]);
    }
}
