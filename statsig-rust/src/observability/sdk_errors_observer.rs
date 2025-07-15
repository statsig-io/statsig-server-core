use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    log_d, log_e,
    networking::{NetworkClient, RequestArgs},
    statsig_metadata::StatsigMetadata,
    OpsStatsEventObserver, StatsigOptions,
};

use super::ops_stats::OpsStatsEvent;

static STATSIG_SDK_EXCEPTION_URL: &str = "https://statsigapi.net/v1/sdk_exception";

fn get_sdk_exception_endpoint() -> String {
    #[cfg(feature = "testing")]
    if let Ok(url) = std::env::var("STATSIG_SDK_EXCEPTION_URL") {
        return url;
    }

    STATSIG_SDK_EXCEPTION_URL.to_string()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBoundaryEvent {
    pub tag: String,
    pub info: String,
    pub exception: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,

    pub bypass_dedupe: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, String>>,
}

const TAG: &str = stringify!(SDKErrorsObserver);
const MAX_SEEN_ERRORS: usize = 1000;

// Observer to post to scrapi when exception happened
// If we never see the exception, log to sdk exception
// TODO: By session end, we flush stats
pub struct SDKErrorsObserver {
    errors_aggregator: RwLock<HashMap<String, u32>>,
    network_client: NetworkClient,
    statsig_options_logging_copy: String,
    sdk_exception_url: String,
}

impl SDKErrorsObserver {
    pub fn new(sdk_key: &str, options: &StatsigOptions) -> Self {
        let mut headers = StatsigMetadata::get_constant_request_headers(sdk_key);
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let options_logging_copy = serde_json::to_string(options).unwrap_or_default();
        SDKErrorsObserver {
            network_client: NetworkClient::new(sdk_key, Some(headers), Some(options))
                .mute_network_error_log(),
            errors_aggregator: RwLock::new(HashMap::new()),
            statsig_options_logging_copy: options_logging_copy,
            sdk_exception_url: get_sdk_exception_endpoint(),
        }
    }

    async fn handle_eb_event(&self, eb_event: ErrorBoundaryEvent) {
        let key = eb_event
            .dedupe_key
            .clone()
            .unwrap_or(format!("{}:{}", eb_event.tag, eb_event.exception));

        let write_guard_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.errors_aggregator.write(),
        )
        .await;

        let mut write_guard = match write_guard_result {
            Ok(guard) => guard,
            Err(_) => {
                log_e!(TAG, "Failed to acquire write lock on errors_aggregator");
                return;
            }
        };

        if write_guard.len() >= MAX_SEEN_ERRORS {
            write_guard.clear();
        }

        let count = write_guard.entry(key).or_default();
        *count += 1;
        if *count > 1 && !eb_event.bypass_dedupe {
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

        log_d!(TAG, "Caught SDK Exception: {}", body);

        let request_args = RequestArgs {
            url: self.sdk_exception_url.clone(),
            retries: 0,
            ..RequestArgs::new()
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
