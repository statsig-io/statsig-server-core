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
            var globalCustomFieldsJson = builder.globalCustomFields != null
                ? JsonConvert.SerializeObject(builder.globalCustomFields)
                : null;
            var specsURLBytes = builder.specsURL != null ? Encoding.UTF8.GetBytes(builder.specsURL) : null;
            var logEventURLBytes = builder.logEventURL != null ? Encoding.UTF8.GetBytes(builder.logEventURL) : null;
            var environmentBytes = builder.environment != null ? Encoding.UTF8.GetBytes(builder.environment) : null;
            var idListsURLBytes = builder.idListsURL != null ? Encoding.UTF8.GetBytes(builder.idListsURL) : null;
            var globalCustomFieldsBytes = globalCustomFieldsJson != null ? Encoding.UTF8.GetBytes(globalCustomFieldsJson) : null;
            unsafe
            {
                fixed (byte* specsURLPtr = specsURLBytes)
                fixed (byte* logEventURLPtr = logEventURLBytes)
                fixed (byte* environmentPtr = environmentBytes)
                fixed (byte* idListsURLPtr = idListsURLBytes)
                fixed (byte* globalCustomFieldsPtr = globalCustomFieldsBytes)
                {
                    _ref = StatsigFFI.statsig_options_create(
                        specsURLPtr,
                        logEventURLPtr,
                        0, // specsAdapterRef
                        0, // eventLoggingAdapterRef
                        environmentPtr,
                        -1, // _eventLoggingFlushIntervalMs
                        builder.eventLoggingMaxQueueSize,
                        builder.specsSyncIntervalMs,
                        null, // outputLogLevel
                        builder.disableCountryLookup ? 1 : 0,
                        builder.waitForCountryLookupInit ? 1 : 0,
                        builder.waitForUserAgentInit ? 1 : 0,
                        builder.enableIDLists ? 1 : 0,
                        builder.disableNetwork ? 1 : 0,
                        idListsURLPtr,
                        builder.idListsSyncIntervalMs,
                        builder.disableAllLogging ? 1 : 0,
                        globalCustomFieldsPtr,
                        0, // observability client ref - not implemented in .NET
                        0, // dataStoreRef - not implemented in .NET
                        builder.initTimeoutMs,
                        builder.fallbackToStatsigApi ? 1 : 0,
                        builder.useThirdPartyUAParser ? 1 : 0,
                        null, // todo: proxy host
                        0, // todo: proxy port
                        null, // todo: proxy auth
                        null // todo: proxy protocol
                    );
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
        public StatsigOptions Build()
        {
            return new StatsigOptions(this);
        }
    }
}