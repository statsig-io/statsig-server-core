package com.statsig;

public class StatsigOptions {
    private volatile String ref;

    private StatsigOptions(Builder builder) {
        this.ref = StatsigJNI.statsigOptionsCreate(
                builder.specsUrl,
                builder.logEventUrl,
                builder.specsSyncIntervalMs,
                builder.eventLoggingFlushIntervalMs,
                builder.eventLoggingMaxQueueSize,
                builder.environment,
                builder.outputLoggerLevel.getValue()
        );

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
        private long specsSyncIntervalMs;
        private long eventLoggingFlushIntervalMs;
        private long eventLoggingMaxQueueSize;
        private String environment;
        private OutputLogger.LogLevel outputLoggerLevel = OutputLogger.LogLevel.WARN;

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

        public Builder setEnvironment(String environment) {
            this.environment = environment;
            return this;
        }

        public StatsigOptions build() {
            return new StatsigOptions(this);
        }
    }
}
