use crate::compression::compression_helper::{compress_data, get_compression_format};
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::log_event_payload::LogEventRequest;
use crate::networking::{NetworkClient, RequestArgs};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::ErrorBoundaryEvent;
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_d, StatsigErr, StatsigRuntime};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

const DEFAULT_LOG_EVENT_URL: &str = "https://prodregistryv2.org/v1/log_event";

#[derive(Deserialize)]
struct LogEventResult {
    success: Option<bool>,
}

const TAG: &str = stringify!(StatsigHttpEventLoggingAdapter);

pub struct StatsigHttpEventLoggingAdapter {
    log_event_url: String,
    network: NetworkClient,
    ops_stats: Arc<OpsStatsForInstance>,
}

impl StatsigHttpEventLoggingAdapter {
    pub fn new(sdk_key: &str, log_event_url: Option<&String>) -> Self {
        let headers = StatsigMetadata::get_constant_request_headers(sdk_key);

        let log_event_url = match log_event_url {
            Some(u) => u,
            _ => DEFAULT_LOG_EVENT_URL,
        }
        .to_string();
        Self {
            log_event_url,
            network: NetworkClient::new(sdk_key, Some(headers)),
            ops_stats: OPS_STATS.get_for_instance(sdk_key),
        }
    }

    pub async fn send_events_over_http(
        &self,
        request: &LogEventRequest,
    ) -> Result<bool, StatsigErr> {
        log_d!(
            TAG,
            "Logging Events ({}): {}",
            &request.event_count,
            json!(&request.payload).to_string()
        );

        let compression_format = get_compression_format();

        // Set headers
        let headers = HashMap::from([
            (
                "statsig-event-count".to_string(),
                request.event_count.to_string(),
            ),
            ("Content-Encoding".to_owned(), compression_format.to_owned()),
            ("Content-Type".to_owned(), "application/json".to_owned()),
        ]);

        // Compress data before sending it
        let bytes = serde_json::to_vec(&request.payload)
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))?;
        let compressed = match compress_data(&bytes) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // Make request
        let response_str = self
            .network
            .post(
                RequestArgs {
                    url: self.log_event_url.clone(),
                    retries: 3,
                    headers: Some(headers),
                    accept_gzip_response: true,
                    ..RequestArgs::new()
                },
                Some(compressed.into()),
            )
            .await
            .map_err(|_err| StatsigErr::NetworkError("Log event failure".into()))?;

        serde_json::from_str::<LogEventResult>(&response_str)
            .map(|result| result.success != Some(false))
            .map_err(|e| {
                StatsigErr::JsonParseError(stringify!(LogEventResult).to_string(), e.to_string())
            })
    }
}

#[async_trait]
impl EventLoggingAdapter for StatsigHttpEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        self.send_events_over_http(&request).await.inspect_err(|_| {
            self.ops_stats.log_error(ErrorBoundaryEvent {
                exception: "LogEventFailed".to_string(),
                tag: "statsig::log_event_failed".to_string(),
                extra: Some(HashMap::from([(
                    "eventCount".to_string(),
                    request.event_count.to_string(),
                )])),
            });
        })
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        true
    }
}

#[cfg(not(feature = "with_zstd"))]
#[tokio::test]
async fn test_event_logging() {
    use crate::log_event_payload::{LogEventPayload, LogEventRequest};
    use std::env;

    let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

    let adapter = StatsigHttpEventLoggingAdapter::new(&sdk_key, None);

    let payload_str = r#"{"events":[{"eventName":"statsig::config_exposure","metadata":{"config":"running_exp_in_unlayered_with_holdout","ruleID":"5suobe8yyvznqasn9Ph1dI"},"secondaryExposures":[{"gate":"global_holdout","gateValue":"false","ruleID":"3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},{"gate":"exp_holdout","gateValue":"false","ruleID":"1rEqLOpCROaRafv7ubGgax"}],"time":1722386636538,"user":{"appVersion":null,"country":null,"custom":null,"customIDs":null,"email":"daniel@statsig.com","ip":null,"locale":null,"privateAttributes":null,"statsigEnvironment":null,"userAgent":null,"userID":"a-user"},"value":null}],"statsigMetadata":{"sdk_type":"statsig-server-core","sdk_version":"0.0.1"}}"#;
    let payload = serde_json::from_str::<LogEventPayload>(payload_str).unwrap();

    let request = LogEventRequest {
        payload,
        event_count: 1,
    };

    let result = adapter.log_events(request).await;

    assert!(result.is_ok(), "Error logging events: {:?}", result.err());
}
