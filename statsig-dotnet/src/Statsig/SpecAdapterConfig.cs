using Newtonsoft.Json;

namespace Statsig
{
    /// <summary>
    /// Configuration for a specs adapter (e.g., gRPC)
    /// </summary>
    public class SpecAdapterConfig
    {
        /// <summary>
        /// The type of adapter to use: "data_store", "network_grpc_websocket", or "network_http"
        /// </summary>
        [JsonProperty("spec_adapter_type")]
        public string AdapterType { get; set; }

        /// <summary>
        /// The URL to fetch specs from (optional, depending on adapter type)
        /// </summary>
        [JsonProperty("spec_adapter_url")]
        public string? SpecsUrl { get; set; }

        /// <summary>
        /// Timeout in milliseconds for initialization (optional)
        /// </summary>
        [JsonProperty("spec_adapter_init_timeout_ms")]
        public int? InitTimeoutMs { get; set; }

        /// <summary>
        /// Authentication mode: "none", "tls", or "mtls"
        /// </summary>
        [JsonProperty("spec_adapter_authentication_mode")]
        public string? AuthenticationMode { get; set; }

        /// <summary>
        /// Path to the CA certificate file for TLS/mTLS
        /// </summary>
        [JsonProperty("spec_adapter_ca_cert_path")]
        public string? CaCertPath { get; set; }

        /// <summary>
        /// Path to the client certificate file for mTLS
        /// </summary>
        [JsonProperty("spec_adapter_client_cert_path")]
        public string? ClientCertPath { get; set; }

        /// <summary>
        /// Path to the client key file for mTLS
        /// </summary>
        [JsonProperty("spec_adapter_client_key_path")]
        public string? ClientKeyPath { get; set; }

        /// <summary>
        /// Domain name for TLS verification
        /// </summary>
        [JsonProperty("spec_adapter_domain_name")]
        public string? DomainName { get; set; }

        public static readonly int DEFAULT_INIT_TIMEOUT_MS = 3000; // default: 3s

        public SpecAdapterConfig(
            string adapterType,
            int? initTimeoutMs = null,
            string? specsUrl = null,
            string? authenticationMode = null,
            string? caCertPath = null,
            string? clientCertPath = null,
            string? clientKeyPath = null,
            string? domainName = null)
        {
            AdapterType = adapterType;
            InitTimeoutMs = initTimeoutMs ?? DEFAULT_INIT_TIMEOUT_MS;
            SpecsUrl = specsUrl;
            AuthenticationMode = authenticationMode;
            CaCertPath = caCertPath;
            ClientCertPath = clientCertPath;
            ClientKeyPath = clientKeyPath;
            DomainName = domainName;
        }
    }

    /// <summary>
    /// Constants for spec adapter types
    /// </summary>
    public static class SpecAdapterType
    {
        public const string DataStore = "data_store";
        public const string NetworkGrpcWebsocket = "network_grpc_websocket";
        public const string NetworkHttp = "network_http";
    }

    /// <summary>
    /// Constants for authentication modes
    /// </summary>
    public static class AuthenticationMode
    {
        public const string None = "none";
        public const string Tls = "tls";
        public const string Mtls = "mtls";
    }
}
