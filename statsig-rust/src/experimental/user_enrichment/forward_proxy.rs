use super::{EnrichmentError, IDListEnrichmentProvider, IdListLookups, IdListMemberships};
use crate::networking::network_error::NetworkError;
use crate::networking::proxy_config::ProxyConfig;
use crate::networking::{NetworkClient, RequestArgs};
use crate::statsig_metadata::StatsigMetadata;
use crate::StatsigErr;
use async_trait::async_trait;
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

const DEFAULT_TIMEOUT_MS: u64 = 100;
const DEFAULT_RETRIES: u32 = 1;
const ENRICHMENT_PATH: &str = "/v1/user_enrichment";

#[derive(Clone, Debug, Default)]
pub struct ForwardProxyEnrichmentOptions {
    /// Base URL of the Statsig forward proxy.
    pub url: String,
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    /// Optional egress proxy and custom CA configuration. This is independent
    /// of the destination forward-proxy URL.
    pub proxy_config: Option<ProxyConfig>,
}

pub struct ForwardProxyUserEnrichmentProvider {
    network: NetworkClient,
    endpoint: String,
    timeout: Duration,
    retries: u32,
    proxy_config: Option<ProxyConfig>,
    ca_cert_pem: Option<Vec<u8>>,
}

impl ForwardProxyUserEnrichmentProvider {
    pub fn new(sdk_key: &str, options: ForwardProxyEnrichmentOptions) -> Result<Self, StatsigErr> {
        let base_url = options.url.trim().trim_end_matches('/');
        if base_url.is_empty() {
            return Err(StatsigErr::InvalidOperation(
                "ForwardProxyEnrichmentOptions.url is required".to_string(),
            ));
        }

        let ca_cert_pem = options
            .proxy_config
            .as_ref()
            .and_then(|config| config.ca_cert_path.as_ref())
            .filter(|path| !path.is_empty())
            .map(std::fs::read)
            .transpose()
            .map_err(|error| {
                StatsigErr::FileError(format!(
                    "Failed to read forward-proxy enrichment CA certificate: {error}"
                ))
            })?;

        let network = NetworkClient::new(
            sdk_key,
            Some(StatsigMetadata::get_constant_request_headers(sdk_key, None)),
            None,
        )
        .mute_network_error_log()
        .fail_fast_on_client_error();

        Ok(Self {
            network,
            endpoint: format!("{base_url}{ENRICHMENT_PATH}"),
            timeout: Duration::from_millis(options.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS)),
            retries: options.retries.unwrap_or(DEFAULT_RETRIES),
            proxy_config: options.proxy_config,
            ca_cert_pem,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[derive(Serialize)]
struct EnrichmentRequest<'a> {
    id_list_lookups: &'a IdListLookups,
}

#[derive(Deserialize)]
struct EnrichmentResponse {
    id_lists: IdListMemberships,
}

#[async_trait]
impl IDListEnrichmentProvider for ForwardProxyUserEnrichmentProvider {
    async fn resolve_id_list_memberships(
        &self,
        lookups: &IdListLookups,
    ) -> Result<IdListMemberships, EnrichmentError> {
        let serialized = serde_json::to_vec(&EnrichmentRequest {
            id_list_lookups: lookups,
        })
        .map_err(|error| EnrichmentError::Transport(error.to_string()))?;
        // Forward proxy accepts gzip, including when the SDK uses zstd for other endpoints.
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&serialized)
            .map_err(|error| EnrichmentError::Transport(error.to_string()))?;
        let body = encoder
            .finish()
            .map_err(|error| EnrichmentError::Transport(error.to_string()))?;

        let request_args = RequestArgs {
            url: self.endpoint.clone(),
            retries: self.retries,
            timeout_ms: self.timeout.as_millis().try_into().unwrap_or(u64::MAX),
            accept_gzip_response: true,
            headers: Some(HashMap::from([
                ("content-type".to_string(), "application/json".to_string()),
                ("content-encoding".to_string(), "gzip".to_string()),
            ])),
            proxy_config: self.proxy_config.clone(),
            ca_cert_pem: self.ca_cert_pem.clone(),
            ..RequestArgs::new()
        };

        let response = self
            .network
            .post(request_args, Some(body))
            .await
            .map_err(|error| map_network_error(error, self.timeout))?;

        let mut data = response
            .data
            .ok_or_else(|| EnrichmentError::Decode("response contained no body".to_string()))?;
        let response: EnrichmentResponse = data
            .deserialize_into()
            .map_err(|error| EnrichmentError::Decode(error.to_string()))?;

        let mut memberships = response.id_lists;
        for lookup in lookups {
            memberships.entry(lookup.clone()).or_default();
        }
        Ok(memberships)
    }

    fn provider_name(&self) -> &'static str {
        "forward_proxy"
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

fn map_network_error(error: NetworkError, timeout: Duration) -> EnrichmentError {
    let status = match &error {
        NetworkError::RequestFailed(_, status, _)
        | NetworkError::RetriesExhausted(_, status, _, _)
        | NetworkError::RequestNotRetryable(_, status, _) => *status,
        _ => None,
    };
    if let Some(status) = status {
        return EnrichmentError::Http(status);
    }

    if matches!(error, NetworkError::RequestTimedOut(_)) {
        EnrichmentError::Timeout(timeout)
    } else {
        EnrichmentError::Transport(error.to_string())
    }
}
