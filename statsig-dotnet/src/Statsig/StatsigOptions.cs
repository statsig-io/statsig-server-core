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
#pragma warning disable IDE0037
            // Build options data object matching Rust's StatsigOptionsData struct
            var optionsData = new
            {
                specs_url = builder.specsURL,
                log_event_url = builder.logEventURL,
                environment = builder.environment,
                specs_sync_interval_ms = builder.specsSyncIntervalMs > 0 ? (int?)builder.specsSyncIntervalMs : null,
                init_timeout_ms = builder.initTimeoutMs > 0 ? (int?)builder.initTimeoutMs : null,
                event_logging_max_queue_size = builder.eventLoggingMaxQueueSize > 0 ? (int?)builder.eventLoggingMaxQueueSize : null,
                disable_country_lookup = builder.disableCountryLookup ? (bool?)true : null,
                wait_for_country_lookup_init = builder.waitForCountryLookupInit ? (bool?)true : null,
                wait_for_user_agent_init = builder.waitForUserAgentInit ? (bool?)true : null,
                enable_id_lists = builder.enableIDLists ? (bool?)true : null,
                disable_network = builder.disableNetwork ? (bool?)true : null,
                id_lists_url = builder.idListsURL,
                id_lists_sync_interval_ms = builder.idListsSyncIntervalMs > 0 ? (int?)builder.idListsSyncIntervalMs : null,
                disable_all_logging = builder.disableAllLogging ? (bool?)true : null,
                global_custom_fields = builder.globalCustomFields,
                fallback_to_statsig_api = builder.fallbackToStatsigApi ? (bool?)true : null,
                use_third_party_ua_parser = builder.useThirdPartyUAParser ? (bool?)true : null,
                proxy_host = builder.proxyConfig?.ProxyHost,
                proxy_port = builder.proxyConfig?.ProxyPort,
                proxy_auth = builder.proxyConfig?.ProxyAuth,
                proxy_protocol = builder.proxyConfig?.ProxyProtocol
            };
#pragma warning disable IDE0037

            var jsonData = JsonConvert.SerializeObject(optionsData, new JsonSerializerSettings
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
        internal string? specsURL;
        internal string? logEventURL;
        internal string? environment;
        internal int specsSyncIntervalMs = -1;
        internal int initTimeoutMs = -1;
        internal bool waitForCountryLookupInit = false;
        internal bool waitForUserAgentInit = false;

        internal bool enableIDLists = false;
        internal string? idListsURL;
        internal int idListsSyncIntervalMs = -1;
        internal int eventLoggingMaxQueueSize = -1;
        internal bool disableCountryLookup = false;
        internal bool disableAllLogging = false;
        internal bool disableNetwork = false;
        internal bool fallbackToStatsigApi = false;
        internal bool useThirdPartyUAParser = false;
        internal Dictionary<string, object>? globalCustomFields;
        internal ProxyConfig? proxyConfig;

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