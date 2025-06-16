using System;
using System.Text;
using System.Text.Json;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the Statsig Server SDK
    /// </summary>
    public class StatsigOptions : IDisposable
    {
        private unsafe ulong _ref;
        internal unsafe ulong Reference => _ref;

        public StatsigOptions(StatsigOptionsBuilder builder)
        {
            var specsURLBytes = builder.specsURL != null ? Encoding.UTF8.GetBytes(builder.specsURL) : null;
            var logEventURLBytes = builder.logEventURL != null ? Encoding.UTF8.GetBytes(builder.logEventURL) : null;
            var environmentBytes = builder.environment != null ? Encoding.UTF8.GetBytes(builder.environment) : null;
            var idListsURLBytes = builder.idListsURL != null ? Encoding.UTF8.GetBytes(builder.idListsURL) : null;
            unsafe
            {
                fixed (byte* specsURLPtr = specsURLBytes)
                fixed (byte* logEventURLPtr = logEventURLBytes)
                fixed (byte* environmentPtr = environmentBytes)
                fixed (byte* idListsURLPtr = idListsURLBytes)
                {
                    _ref = StatsigFFI.statsig_options_create(
                        specsURLPtr,
                        logEventURLPtr,
                        0, // specsAdapterRef
                        0, // eventLoggingAdapterRef
                        environmentPtr,
                        -1, // _eventLoggingFlushIntervalMs
                        -1, // eventLoggingMaxQueueSize
                        builder.specsSyncIntervalMs,
                        null, // outputLogLevel
                        -1, // disableCountryLookup
                        -1, // disableUserAgentParsing
                        builder.waitForCountryLookupInit ? 1 : 0, // waitForCountryLookupInit
                        builder.waitForUserAgentInit ? 1 : 0,
                        builder.enableIDLists ? 1 : 0,
                        idListsURLPtr,
                        builder.idListsSyncIntervalMs);
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
        internal bool waitForCountryLookupInit = false;
        internal bool waitForUserAgentInit = false;

        internal bool enableIDLists = false;
        internal string? idListsURL;
        internal int idListsSyncIntervalMs = -1;

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
        public StatsigOptions Build()
        {
            return new StatsigOptions(this);
        }
    }
}