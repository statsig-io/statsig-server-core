package com.statsig;

public class StatsigOptions {
    private volatile String ref;

    private StatsigOptions(Builder builder) {
        this.ref = StatsigJNI.statsigOptionsCreate(
                builder.specsUrl,
                builder.logEventUrl,
                builder.idListsUrl,
                builder.idListsSyncIntervalMs,
                builder.specsSyncIntervalMs,
                builder.eventLoggingFlushIntervalMs,
                builder.eventLoggingMaxQueueSize,
                builder.initTimeoutMs,
                builder.environment,
                builder.outputLoggerLevel.getValue(),
                builder.serviceName,
                builder.observabilityClient,
                builder.enableIDLists,
                builder.enableCountryLookup,
                builder.disableAllLogging,
                builder.enableUserAgentParsing,
                builder.disableNetwork);

        ResourceCleaner.register(this, () -> {
            if (ref != null) {
                StatsigJNI.statsigOptionsRelease(ref);
                ref = null;
            }
        });
    }

    String getRef() {
        return ref;
    }

    public static class Builder {
        private String specsUrl;
        private String logEventUrl;
        private String idListsUrl;
        private long idListsSyncIntervalMs;
        private long specsSyncIntervalMs;
        private long eventLoggingFlushIntervalMs;
        private long eventLoggingMaxQueueSize;
        private long initTimeoutMs;
        private String environment;
        private ObservabilityClient observabilityClient;
        private boolean enableIDLists = false;
        private boolean waitForUserAgentParsingInit = false;
        private boolean waitForCountryLookupInit = false;
        private boolean disableAllLogging = false;
        private boolean disableNetwork = false;
        private OutputLogger.LogLevel outputLoggerLevel = OutputLogger.LogLevel.WARN;
        private String serviceName;

        public Builder setOutputLoggerLevel(OutputLogger.LogLevel level) {
            this.outputLoggerLevel = level;
            OutputLogger.logLevel = level;
            return this;
        }

        public Builder setSpecsUrl(String specsUrl) {
            this.specsUrl = specsUrl;
            return this;
        }

        public Builder setLogEventUrl(String logEventUrl) {
            this.logEventUrl = logEventUrl;
            return this;
        }

        public Builder setSpecsSyncIntervalMs(long specsSyncIntervalMs) {
            this.specsSyncIntervalMs = specsSyncIntervalMs;
            return this;
        }

        public Builder setEventLoggingFlushIntervalMs(long eventLoggingFlushIntervalMs) {
            this.eventLoggingFlushIntervalMs = eventLoggingFlushIntervalMs;
            return this;
        }

        public Builder setEventLoggingMaxQueueSize(long eventLoggingMaxQueueSize) {
            this.eventLoggingMaxQueueSize = eventLoggingMaxQueueSize;
            return this;
        }

        public Builder setInitTimeoutMs(long initTimeoutMs) {
            this.initTimeoutMs = initTimeoutMs;
            return this;
        }

        public Builder setEnvironment(String environment) {
            this.environment = environment;
            return this;
        }

        public Builder setDisableAllLogging(boolean disableAllLogging) {
            this.disableAllLogging = disableAllLogging;
            return this;
        }
        
        public Builder setObservabilityClient(ObservabilityClient observabilityClient) {
            this.observabilityClient = observabilityClient;
            return this;
        }

        public Builder setIdListsUrl(String idListsUrl) {
            this.idListsUrl = idListsUrl;
            return this;
        }

        public Builder setIdListsSyncIntervalMs(long idListsSyncIntervalMs) {
            this.idListsSyncIntervalMs = idListsSyncIntervalMs;
            return this;
        }

        public Builder setEnableIDLists(boolean enableIDLists) {
            this.enableIDLists = enableIDLists;
            return this;
        }

        public Builder setServiceName(String serviceName) {
            this.serviceName = serviceName;
            return this;
        }

        public Builder setWaitForUserAgentParsingInit(boolean waitForUserAgentParsingInit) {
            this.waitForUserAgentParsingInit = waitForUserAgentParsingInit;
            return this;
        }

        public Builder setWaitForCountryLookupInit(boolean waitForCountryLookupInit) {
            this.waitForCountryLookupInit = waitForCountryLookupInit;
            return this;
        }

        public Builder setDisableNetwork(boolean disableNetwork) {
            this.disableNetwork = disableNetwork;
            return this;
        }

        public StatsigOptions build() {
            return new StatsigOptions(this);
        }
    }
}
