package com.statsig;

public class StatsigOptions implements AutoCloseable {
    private final int ref;

    private StatsigOptions(Builder builder) {
        this.ref = StatsigJNI.statsigOptionsCreate(
                builder.specsUrl,
                builder.logEventUrl,
                builder.specsSyncIntervalMs,
                builder.eventLoggingFlushIntervalMs,
                builder.eventLoggingMaxQueueSize,
                builder.environment
        );
    }

    int getRef() {
        return ref;
    }

    @Override
    public void close() {
        if (ref != 0) {
            StatsigJNI.statsigOptionsRelease(ref);
        }
    }

    public static class Builder {
        private String specsUrl;
        private String logEventUrl;
        private long specsSyncIntervalMs;
        private long eventLoggingFlushIntervalMs;
        private long eventLoggingMaxQueueSize;
        private String environment;

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
