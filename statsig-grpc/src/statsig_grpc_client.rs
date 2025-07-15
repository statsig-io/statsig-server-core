use crate::statsig_forward_proxy::config_spec_request::ApiVersion;
use crate::statsig_forward_proxy::statsig_forward_proxy_client::StatsigForwardProxyClient;
use crate::statsig_forward_proxy::{ConfigSpecRequest, ConfigSpecResponse};
use crate::statsig_grpc_err::StatsigGrpcErr;
use parking_lot::Mutex;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tonic::Streaming;

pub struct StatsigGrpcClient {
    sdk_key: String,
    proxy_api: String,
    grpc_client: Mutex<Option<StatsigForwardProxyClient<Channel>>>,
    tls_config: Option<ClientTlsConfig>,
}

impl StatsigGrpcClient {
    pub fn new(
        sdk_key: &str,
        proxy_api: &str,
        authentication_mode: Option<String>,
        ca_cert_path: Option<String>,
        client_cert_path: Option<String>,
        client_key_path: Option<String>,
        domain_name: Option<String>,
    ) -> Self {
        Self {
            sdk_key: sdk_key.to_string(),
            proxy_api: proxy_api.to_string(),
            tls_config: Self::setup_tls_client(
                authentication_mode,
                ca_cert_path,
                client_cert_path,
                client_key_path,
                domain_name,
                proxy_api,
            ),
            grpc_client: Mutex::new(None),
        }
    }

    pub async fn connect_client(&self) -> Result<(), StatsigGrpcErr> {
        self.get_or_setup_grpc_client().await.map(|_| ())
    }

    pub fn reset_client(&self) {
        match self.grpc_client.try_lock_for(Duration::from_secs(1)) {
            Some(mut lock) => {
                *lock = None;
            }
            None => {
                eprintln!("Failed to reset grpc client");
            }
        };
    }

    pub async fn get_specs(
        &self,
        lcut: Option<u64>,
        zstd_dict_id: Option<String>,
    ) -> Result<ConfigSpecResponse, StatsigGrpcErr> {
        let request = create_config_spec_request(&self.sdk_key, lcut, zstd_dict_id);
        let mut client = self.get_or_setup_grpc_client().await?;

        client
            .get_config_spec(request)
            .await
            .map_err(StatsigGrpcErr::ErrorGrpcStatus)
            .map(|r| r.into_inner())
    }

    pub async fn get_specs_stream(
        &self,
        lcut: Option<u64>,
        zstd_dict_id: Option<String>,
    ) -> Result<Streaming<ConfigSpecResponse>, StatsigGrpcErr> {
        let request = create_config_spec_request(&self.sdk_key, lcut, zstd_dict_id);
        let mut client = self.get_or_setup_grpc_client().await?;

        client
            .stream_config_spec(request)
            .await
            .map_err(StatsigGrpcErr::ErrorGrpcStatus)
            .map(|s| s.into_inner())
    }

    fn setup_tls_client(
        authentication_mode: Option<String>,
        ca_cert_path: Option<String>,
        client_cert_path: Option<String>,
        client_key_path: Option<String>,
        domain_name: Option<String>,
        proxy_api: &str,
    ) -> Option<ClientTlsConfig> {
        let domain_name = domain_name.unwrap_or_else(|| {
            Self::extract_host(proxy_api)
                .unwrap_or_default()
                .to_string()
        });
        match authentication_mode
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("tls") => {
                let ca_cert_path = ca_cert_path?;
                let ca_cert: Vec<u8> = std::fs::read(ca_cert_path).ok()?;
                let ca_cert = Certificate::from_pem(ca_cert);

                Some(
                    ClientTlsConfig::new()
                        .ca_certificate(ca_cert)
                        .domain_name(domain_name), // <-- adjust this as needed
                )
            }
            Some("mtls") => {
                let ca_cert_path = ca_cert_path?;
                let client_cert_path = client_cert_path?;
                let client_key_path = client_key_path?;

                let ca_cert = std::fs::read(ca_cert_path).ok()?;
                let client_cert = std::fs::read(client_cert_path).ok()?;
                let client_key = std::fs::read(client_key_path).ok()?;

                let ca_cert = Certificate::from_pem(ca_cert);
                let identity = Identity::from_pem(client_cert, client_key);

                Some(
                    ClientTlsConfig::new()
                        .ca_certificate(ca_cert)
                        .identity(identity)
                        .domain_name(domain_name), // <-- adjust this as needed
                )
            }
            _ => None,
        }
    }

    fn extract_host(url: &str) -> Option<&str> {
        // Strip scheme if present
        let without_scheme = if let Some(pos) = url.find("://") {
            &url[(pos + 3)..]
        } else {
            url
        };

        // Split off path/query/fragment after the host[:port]
        let host_port = without_scheme.split('/').next()?; // First part is host[:port]

        // Split off port if present
        host_port.split(':').next()
    }

    async fn get_or_setup_grpc_client(
        &self,
    ) -> Result<StatsigForwardProxyClient<Channel>, StatsigGrpcErr> {
        {
            let lock = self
                .grpc_client
                .try_lock_for(Duration::from_secs(1))
                .ok_or(StatsigGrpcErr::FailedToGetLock)?;

            if let Some(client) = lock.as_ref() {
                return Ok(client.clone());
            }
        }

        let mut channel_builder = Channel::from_shared(self.proxy_api.clone())
            .map_err(|e| StatsigGrpcErr::FailedToConnect(e.to_string()))?
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(Duration::from_secs(30));

        if let Some(tls_config) = self.tls_config.clone() {
            channel_builder = channel_builder
                .tls_config(tls_config)
                .map_err(|e| StatsigGrpcErr::Authentication(e.to_string()))?;
        }
        let channel = channel_builder
            .connect()
            .await
            .map_err(|e| StatsigGrpcErr::FailedToConnect(e.to_string()))?;

        let new_client = StatsigForwardProxyClient::new(channel);

        let mut lock = self
            .grpc_client
            .try_lock_for(Duration::from_secs(1))
            .ok_or(StatsigGrpcErr::FailedToGetLock)?;

        *lock = Some(new_client.clone());
        Ok(new_client)
    }
}

fn create_config_spec_request(
    sdk_key: &str,
    current_lcut: Option<u64>,
    current_zstd_dict_id: Option<String>,
) -> ConfigSpecRequest {
    ConfigSpecRequest {
        since_time: current_lcut,
        sdk_key: sdk_key.to_string(),
        version: Some(ApiVersion::V2 as i32),
        zstd_dict_id: current_zstd_dict_id,
    }
}
