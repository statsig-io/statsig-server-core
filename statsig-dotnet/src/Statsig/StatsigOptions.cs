using System;
using System.Text;
using System.Text.Json;
using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the Statsig Server SDK
    /// </summary>
    public class StatsigOptions : IDisposable
    {
        private readonly unsafe ulong _ref;
        internal unsafe ulong Reference => _ref;

        public StatsigOptions(StatsigOptionsBuilder builder)
        {
            var jsonData = JsonConvert.SerializeObject(builder, new JsonSerializerSettings
            {
                NullValueHandling = NullValueHandling.Ignore
            });
            var jsonBytes = Encoding.UTF8.GetBytes(jsonData);

            unsafe
            {
                fixed (byte* jsonPtr = jsonBytes)
                {
                    _ref = StatsigFFI.statsig_options_create_from_data(jsonPtr);
                }
            }
        }

        ~StatsigOptions()
        {
            Dispose(false);
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            StatsigFFI.statsig_options_release(_ref);
        }
    }

    public class StatsigOptionsBuilder
    {
        [JsonProperty("specs_url")]
        internal string? specsURL;

        [JsonProperty("log_event_url")]
        internal string? logEventURL;

        [JsonProperty("environment")]
        internal string? environment;

        [JsonProperty("specs_sync_interval_ms")]
        internal int? specsSyncIntervalMs;

        [JsonProperty("init_timeout_ms")]
        internal int? initTimeoutMs;

        [JsonProperty("wait_for_country_lookup_init")]
        internal bool? waitForCountryLookupInit;

        [JsonProperty("wait_for_user_agent_init")]
        internal bool? waitForUserAgentInit;

        [JsonProperty("enable_id_lists")]
        internal bool? enableIDLists;

        [JsonProperty("id_lists_url")]
        internal string? idListsURL;

        [JsonProperty("id_lists_sync_interval_ms")]
        internal int? idListsSyncIntervalMs;

        [JsonProperty("event_logging_max_queue_size")]
        internal int? eventLoggingMaxQueueSize;

        [JsonProperty("disable_country_lookup")]
        internal bool? disableCountryLookup;

        [JsonProperty("disable_all_logging")]
        internal bool? disableAllLogging;

        [JsonProperty("disable_network")]
        internal bool? disableNetwork;

        [JsonProperty("fallback_to_statsig_api")]
        internal bool? fallbackToStatsigApi;

        [JsonProperty("use_third_party_ua_parser")]
        internal bool? useThirdPartyUAParser;

        [JsonProperty("global_custom_fields")]
        internal Dictionary<string, object>? globalCustomFields;

        // START: Flattened spec adapter config fields for serialization
        [JsonIgnore]
        internal SpecAdapterConfig? specAdapterConfig;


        [JsonProperty("spec_adapter_type")]
        internal string? SpecAdapterType => specAdapterConfig?.AdapterType;

        [JsonProperty("spec_adapter_specs_url")]
        internal string? SpecAdapterSpecsUrl => specAdapterConfig?.SpecsUrl;

        [JsonProperty("spec_adapter_init_timeout_ms")]
        internal int? SpecAdapterInitTimeoutMs => specAdapterConfig?.InitTimeoutMs;

        [JsonProperty("spec_adapter_authentication_mode")]
        internal string? SpecAdapterAuthenticationMode => specAdapterConfig?.AuthenticationMode;

        [JsonProperty("spec_adapter_ca_cert_path")]
        internal string? SpecAdapterCaCertPath => specAdapterConfig?.CaCertPath;

        [JsonProperty("spec_adapter_client_cert_path")]
        internal string? SpecAdapterClientCertPath => specAdapterConfig?.ClientCertPath;

        [JsonProperty("spec_adapter_client_key_path")]
        internal string? SpecAdapterClientKeyPath => specAdapterConfig?.ClientKeyPath;

        [JsonProperty("spec_adapter_domain_name")]
        internal string? SpecAdapterDomainName => specAdapterConfig?.DomainName;
        // END: Flattened spec adapter config fields for serialization

        // START: Flattened proxy config fields for serialization
        [JsonIgnore]
        internal ProxyConfig? proxyConfig;

        [JsonProperty("proxy_host")]
        internal string? ProxyHost => proxyConfig?.ProxyHost;

        [JsonProperty("proxy_port")]
        internal int? ProxyPort => proxyConfig?.ProxyPort;

        [JsonProperty("proxy_auth")]
        internal string? ProxyAuth => proxyConfig?.ProxyAuth;

        [JsonProperty("proxy_protocol")]
        internal string? ProxyProtocol => proxyConfig?.ProxyProtocol;
        // END: Flattened proxy config fields for serialization

        public StatsigOptionsBuilder SetSpecsURL(string specsURL)
        {
            this.specsURL = specsURL;
            return this;
        }

        public StatsigOptionsBuilder SetLogEventURL(string logEventURL)
        {
            this.logEventURL = logEventURL;
            return this;
        }

        public StatsigOptionsBuilder SetInitTimeoutMs(int initTimeoutMs)
        {
            this.initTimeoutMs = initTimeoutMs;
            return this;
        }

        public StatsigOptionsBuilder SetFallbackToStatsigApi(bool fallbackToStatsigApi)
        {
            this.fallbackToStatsigApi = fallbackToStatsigApi;
            return this;
        }

        public StatsigOptionsBuilder SetEnvironment(string environment)
        {
            this.environment = environment;
            return this;
        }

        public StatsigOptionsBuilder SetSpecsSyncIntervalMs(int specsSyncIntervalMs)
        {
            this.specsSyncIntervalMs = specsSyncIntervalMs;
            return this;
        }

        public StatsigOptionsBuilder SetEventLoggingMaxQueueSize(int eventLoggingMaxQueueSize)
        {
            this.eventLoggingMaxQueueSize = eventLoggingMaxQueueSize;
            return this;
        }
        public StatsigOptionsBuilder SetWaitForCountryLookupInit(bool waitForCountryLookupInit)
        {
            this.waitForCountryLookupInit = waitForCountryLookupInit;
            return this;
        }

        public StatsigOptionsBuilder SetWaitForUserAgentInit(bool waitForUserAgentInit)
        {
            this.waitForUserAgentInit = waitForUserAgentInit;
            return this;
        }

        public StatsigOptionsBuilder SetDisableCountryLookup(bool disableCountryLookup)
        {
            this.disableCountryLookup = disableCountryLookup;
            return this;
        }

        public StatsigOptionsBuilder SetDisableNetwork(bool disableNetwork)
        {
            this.disableNetwork = disableNetwork;
            return this;
        }

        public StatsigOptionsBuilder SetDisableAllLogging(bool disableAllLogging)
        {
            this.disableAllLogging = disableAllLogging;
            return this;
        }
        public StatsigOptionsBuilder SetEnableIDLists(bool enableIDLists)
        {
            this.enableIDLists = enableIDLists;
            return this;
        }
        public StatsigOptionsBuilder SetIDListsURL(string idListsURL)
        {
            this.idListsURL = idListsURL;
            return this;
        }
        public StatsigOptionsBuilder SetIDListsSyncIntervalMs(int idListsSyncIntervalMs)
        {
            this.idListsSyncIntervalMs = idListsSyncIntervalMs;
            return this;
        }
        public StatsigOptionsBuilder SetUseThirdPartyUAParser(bool useThirdPartyUAParser)
        {
            this.useThirdPartyUAParser = useThirdPartyUAParser;
            return this;
        }
        public StatsigOptionsBuilder SetGlobalCustomFields(Dictionary<string, object> globalCustomFields)
        {
            this.globalCustomFields = globalCustomFields;
            return this;
        }

        public StatsigOptionsBuilder SetSpecAdapterConfig(SpecAdapterConfig specAdapterConfig)
        {
            this.specAdapterConfig = specAdapterConfig;
            return this;
        }

        public StatsigOptionsBuilder SetProxyConfig(ProxyConfig proxyConfig)
        {
            this.proxyConfig = proxyConfig;
            return this;
        }

        public StatsigOptions Build()
        {
            return new StatsigOptions(this);
        }
    }
}