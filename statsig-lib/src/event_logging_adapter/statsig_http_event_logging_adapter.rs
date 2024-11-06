use crate::event_logging_adapter::{EventLoggingAdapter, LogEventRequest};
use crate::network_client::{NetworkClient, RequestArgs};
use crate::statsig_metadata::StatsigMetadata;
use crate::{log_d, StatsigErr};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

const DEFAULT_LOG_EVENT_URL: &str = "https://prodregistryv2.org/v1/log_event";

#[derive(Deserialize)]
struct LogEventResult {
    success: bool,
}

pub struct StatsigHttpEventLoggingAdapter {
    log_event_url: String,
    network: NetworkClient,
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
            network: NetworkClient::new(Some(headers)),
        }
    }

    pub async fn send_events_over_http(
        &self,
        request: LogEventRequest,
    ) -> Result<bool, StatsigErr> {
        log_d!(
            "Logging Events ({}): {}",
            &request.event_count,
            json!(&request.payload).to_string()
        );

        let response_str = self
            .network
            .post(RequestArgs {
                url: self.log_event_url.clone(),
                retries: 3,
                body: Some(json!(request.payload)),
                accept_gzip_response: true,
                ..RequestArgs::new()
            })
            .ok_or(StatsigErr::NetworkError("No result from network".into()))?;

        serde_json::from_str::<LogEventResult>(&response_str)
            .map(|result| result.success)
            .map_err(|e| {
                StatsigErr::JsonParseError(stringify!(LogEventResult).to_string(), e.to_string())
            })
    }
}

#[async_trait]
impl EventLoggingAdapter for StatsigHttpEventLoggingAdapter {
    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        self.send_events_over_http(request).await
    }
}

#[tokio::test]
async fn test_event_logging() {
    use super::LogEventPayload;
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

    assert!(result.is_ok());
}
