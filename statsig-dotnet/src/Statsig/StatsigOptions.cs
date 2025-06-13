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
            unsafe
            {
                fixed (byte* specsURLPtr = specsURLBytes)
                fixed (byte* logEventURLPtr = logEventURLBytes)
                fixed (byte* environmentPtr = environmentBytes)
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
                        -1, // waitForCountryLookupInit
                        -1); // waitForUserAgentInit
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

        public StatsigOptions Build()
        {
            return new StatsigOptions(this);
        }
    }
}