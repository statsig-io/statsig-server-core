use crate::compression::compression_helper::{compress_data, get_compression_format};
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::log_event_payload::LogEventRequest;
use crate::networking::{NetworkClient, NetworkError, RequestArgs};
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_d, StatsigErr, StatsigOptions, StatsigRuntime};
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
}

impl StatsigHttpEventLoggingAdapter {
    #[must_use]
    pub fn new(sdk_key: &str, options: Option<&StatsigOptions>) -> Self {
        let headers = StatsigMetadata::get_constant_request_headers(sdk_key);

        let log_event_url = options
            .and_then(|opts| opts.log_event_url.as_ref())
            .map(|u| u.to_string())
            .unwrap_or_else(|| DEFAULT_LOG_EVENT_URL.to_string());

        Self {
            log_event_url,
            network: NetworkClient::new(sdk_key, Some(headers), options),
        }
    }

    pub async fn send_events_over_http(&self, request: &LogEventRequest) -> Result<(), StatsigErr> {
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
            (
                "statsig-retry-count".to_string(),
                request.retries.to_string(),
            ),
            ("Content-Encoding".to_owned(), compression_format.clone()),
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
        let response = self
            .network
            .post(
                RequestArgs {
                    url: self.log_event_url.clone(),
                    headers: Some(headers),
                    accept_gzip_response: true,
                    ..RequestArgs::new()
                },
                Some(compressed),
            )
            .await
            .map_err(StatsigErr::NetworkError)?;

        let mut res_data = match response.data {
            Some(data) => data,
            None => {
                return Err(StatsigErr::NetworkError(NetworkError::RequestFailed(
                    self.log_event_url.clone(),
                    response.status_code,
                    "Empty response from network".to_string(),
                )));
            }
        };

        let result = res_data
            .deserialize_into::<LogEventResult>()
            .map(|result| result.success != Some(false))
            .map_err(|e| {
                StatsigErr::JsonParseError(stringify!(LogEventResult).to_string(), e.to_string())
            })?;

        if result {
            Ok(())
        } else {
            Err(StatsigErr::LogEventError(
                "Unsuccessful response from network".into(),
            ))
        }
    }
}

#[async_trait]
impl EventLoggingAdapter for StatsigHttpEventLoggingAdapter {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        match self.send_events_over_http(&request).await {
            Ok(()) => Ok(true),
            Err(e) => Err(e),
        }
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
        retries: 0,
    };

    let result = adapter.log_events(request).await;

    assert!(result.is_ok(), "Error logging events: {:?}", result.err());
}
