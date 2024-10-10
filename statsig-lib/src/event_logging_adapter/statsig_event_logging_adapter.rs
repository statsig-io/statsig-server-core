use crate::network_client::{NetworkClient, RequestArgs};
use crate::statsig_metadata::StatsigMetadata;
use crate::statsig_options::StatsigOptions;
use crate::{log_d, log_e, unwrap_or_return};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::event_logging_adapter::EventLoggingAdapter;

const DEFAULT_LOG_EVENT_URL: &str = "https://prodregistryv2.org/v1/log_event";

#[derive(Deserialize)]
struct LogEventResult {
    success: bool,
}

struct AdapterContext {
    log_event_url: String,
    network: NetworkClient,
}

pub struct StatsigEventLoggingAdapter {
    context: RwLock<Option<Arc<AdapterContext>>>,
}

impl StatsigEventLoggingAdapter {
    pub fn new() -> Self {
        Self {
            context: RwLock::new(None),
        }
    }
}

fn get_request_headers(sdk_key: &str) -> HashMap<String, String> {
    let metadata = StatsigMetadata::new();
    vec![
        ("STATSIG-API-KEY".to_string(), sdk_key.to_string()),
        (
            "STATSIG-SDK-TYPE".to_string(),
            metadata.sdk_type.to_string(),
        ),
        (
            "STATSIG-SDK-VERSION".to_string(),
            metadata.sdk_version.to_string(),
        ),
    ]
    .into_iter()
    .collect()
}

#[async_trait]
impl EventLoggingAdapter for StatsigEventLoggingAdapter {
    fn bind(&self, sdk_key: &str, options: &StatsigOptions) {
        match self.context.write() {
            Ok(mut context) => {
                let url = options
                    .log_event_url
                    .clone()
                    .unwrap_or_else(|| DEFAULT_LOG_EVENT_URL.to_string())
                    .to_string();

                *context = Some(Arc::new(AdapterContext {
                    log_event_url: url,
                    network: NetworkClient::new(Some(get_request_headers(sdk_key))),
                }));
            }
            Err(e) => log_e!("Failed to acquire write lock: {}", e),
        }
    }

    async fn log_events(
        &self,
        payload: HashMap<String, Value>,
        event_count: usize,
    ) -> Result<bool, String> {
        let opt_context = match self.context.read() {
            Ok(c) => c.clone(),
            Err(e) => return Err(format!("Failed to acquire read lock: {}", e)),
        };

        let context = match opt_context {
            Some(c) => c,
            None => return Err("Logging adapter not setup".to_string()),
        };
        log_d!("Logging Events ({}): {}", event_count, json!(payload).to_string());

        let response = context
            .network
            .post(RequestArgs {
                url: context.log_event_url.clone(),
                retries: 3,
                body: Some(payload),
                ..RequestArgs::new()
            })
            .await;

        let response_str = unwrap_or_return!(response, Err("No result from network".to_string()));

        match serde_json::from_str::<LogEventResult>(&response_str) {
            Ok(parsed) => Ok(parsed.success),
            _ => Err("No result from network".to_string()),
        }
    }
}



#[tokio::test]
async fn test_event_logging() {
    use std::env;

    let adapter = StatsigEventLoggingAdapter::new();
    let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

    adapter.bind(
        &sdk_key,
        &StatsigOptions::new(),
    );

    let payload_str = r#"{"events":[{"eventName":"statsig::config_exposure","metadata":{"config":"running_exp_in_unlayered_with_holdout","ruleID":"5suobe8yyvznqasn9Ph1dI"},"secondaryExposures":[{"gate":"global_holdout","gateValue":"false","ruleID":"3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},{"gate":"exp_holdout","gateValue":"false","ruleID":"1rEqLOpCROaRafv7ubGgax"}],"time":1722386636538,"user":{"appVersion":null,"country":null,"custom":null,"customIDs":null,"email":"daniel@statsig.com","ip":null,"locale":null,"privateAttributes":null,"statsigEnvironment":null,"userAgent":null,"userID":"a-user"},"value":null}],"statsigMetadata":{"sdk_type":"statsig-server-core","sdk_version":"0.0.1"}}"#;
    let payload = serde_json::from_str::<HashMap<String, Value>>(payload_str).unwrap();

    let result = adapter.log_events(payload, 0).await;

    assert!(result.is_ok());
}
