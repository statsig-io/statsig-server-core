use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    networking::{NetworkClient, RequestArgs},
    statsig_metadata::StatsigMetadata,
    OpsStatsEventObserver, StatsigOptions,
};

use super::ops_stats::OpsStatsEvent;

static SDK_EXCEPTION_ENDPOINT: &str = "https://statsigapi.net/v1/sdk_exception";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBoundaryEvent {
    pub tag: String,
    pub exception: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, String>>,
}

// Observer to post to scrapi when exception happened
// If we never see the exception, log to sdk exception
// TODO: By session end, we flush stats
pub struct SDKErrorsObserver {
    errors_aggregator: RwLock<HashMap<String, u32>>,
    network_client: NetworkClient,
    statsig_options_logging_copy: String,
}

impl SDKErrorsObserver {
    pub fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        let options_logging_copy = serde_json::to_string(options).unwrap_or_default();
        SDKErrorsObserver {
            network_client: NetworkClient::new(
                sdk_key,
                Some(StatsigMetadata::get_constant_request_headers(sdk_key)),
                options.disable_network,
            )
            .mute_network_error_log(),
            errors_aggregator: RwLock::new(HashMap::new()),
            statsig_options_logging_copy: options_logging_copy,
        }
    }

    async fn handle_eb_event(&self, eb_event: ErrorBoundaryEvent) {
        let key = format!("{}:{}", eb_event.tag, eb_event.exception);
        let mut write_guard = self.errors_aggregator.write().await;
        let count = write_guard.entry(key).or_default();
        *count += 1;
        if *count > 1 {
            return;
        }
        self.log_exception(eb_event).await;
    }

    async fn log_exception(&self, e: ErrorBoundaryEvent) {
        let mut body_obj = serde_json::to_value(e).unwrap_or_default();
        if let Value::Object(ref mut map) = body_obj {
            map.insert(
                "statsigOptions".to_string(),
                Value::String(self.statsig_options_logging_copy.clone()),
            );
        }
        let body = serde_json::to_string_pretty(&body_obj).unwrap_or_default();
        let request_args = RequestArgs {
            url: SDK_EXCEPTION_ENDPOINT.to_owned(),
            retries: 0,
            ..RequestArgs::default()
        };
        let _ = self
            .network_client
            .post(request_args, Some(body.into()))
            .await;
    }
}

#[async_trait]
impl OpsStatsEventObserver for SDKErrorsObserver {
    async fn handle_event(&self, event: OpsStatsEvent) {
        if let OpsStatsEvent::SDKError(e) = event {
            self.handle_eb_event(e).await;
        }
    }
}
