package com.statsig;

public class StatsigOptions {
  private static class CleaningAction implements Runnable {
    private final long ref;

    CleaningAction(long ref) {
      this.ref = ref;
    }

    @Override
    public void run() {
      StatsigJNI.statsigOptionsRelease(ref);
    }
  }

  private volatile long ref;

  private StatsigOptions(Builder builder) {
    /**
     * WARNING: The order of parameters in this method **MUST MATCH EXACTLY** with the corresponding
     * Rust implementation in `statsig_options_jni.rs`. Any mismatch will cause incorrect values to
     * be passed across the JNI boundary.
     */
    this.ref =
        StatsigJNI.statsigOptionsCreate(
            builder.specsUrl,
            builder.logEventUrl,
            builder.idListsUrl,
            builder.idListsSyncIntervalMs,
            builder.specsSyncIntervalMs,
            builder.eventLoggingFlushIntervalMs,
            builder.eventLoggingMaxQueueSize,
            builder.eventLoggingMaxPendingBatchQueueSize,
            builder.initTimeoutMs,
            builder.environment,
            builder.outputLoggerLevel.getValue(),
            builder.serviceName,
            builder.observabilityClient,
            builder.dataStore,
            builder.outputLoggerProvider,
            builder.proxyConfig,
            builder.enableIDLists,
            builder.waitForCountryLookupInit,
            builder.disableAllLogging,
            builder.waitForUserAgentInit,
            builder.disableNetwork,
            builder.disableUserCountryLookup,
            builder.fallbackToStatsigApi,
            builder.useThirdPartyUAParser);

    ResourceCleaner.register(this, new CleaningAction(ref));
  }

  long getRef() {
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
    private long eventLoggingMaxPendingBatchQueueSize;
    private long initTimeoutMs;
    private String environment;
    private ObservabilityClient observabilityClient;
    private DataStore dataStore;
    private OutputLoggerProvider outputLoggerProvider;
    private ProxyConfig proxyConfig;
    private boolean enableIDLists = false;
    private boolean waitForUserAgentInit = false;
    private boolean waitForCountryLookupInit = false;
    private boolean disableUserCountryLookup = false;
    private boolean disableAllLogging = false;
    private boolean disableNetwork = false;
    private boolean fallbackToStatsigApi = false;
    private OutputLogger.LogLevel outputLoggerLevel = OutputLogger.LogLevel.WARN;
    private String serviceName;
    private boolean useThirdPartyUAParser;

    public Builder setOutputLoggerLevel(OutputLogger.LogLevel level) {
      this.outputLoggerLevel = level;
      OutputLogger.logLevel = level;
      return this;
    }

    public Builder setSpecsUrl(String specsUrl) {
      this.specsUrl = specsUrl;
      return this;
    }

    public Builder setProxyConfig(ProxyConfig proxyConfig) {
      this.proxyConfig = proxyConfig;
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

    public Builder setEventLoggingMaxPendingBatchQueueSize(
        long eventLoggingMaxPendingBatchQueueSize) {
      this.eventLoggingMaxPendingBatchQueueSize = eventLoggingMaxPendingBatchQueueSize;
      return this;
    }

    public Builder setDataStore(DataStore dataStore) {
      this.dataStore = dataStore;
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

    public Builder setOutputLoggerProvider(OutputLoggerProvider outputLoggerProvider) {
      this.outputLoggerProvider = outputLoggerProvider;
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

    public Builder setWaitForUserAgentInit(boolean waitForUserAgentInit) {
      this.waitForUserAgentInit = waitForUserAgentInit;
      return this;
    }

    public Builder setWaitForCountryLookupInit(boolean waitForCountryLookupInit) {
      this.waitForCountryLookupInit = waitForCountryLookupInit;
      return this;
    }

    public Builder setFallbackToStatsigApi(boolean fallbackToStatsigApi) {
      this.fallbackToStatsigApi = fallbackToStatsigApi;
      return this;
    }

    public Builder setDisableUserCountryLookup(boolean disableUserCountryLookup) {
      this.disableUserCountryLookup = disableUserCountryLookup;
      return this;
    }

    public Builder setUseThirdPartyUAParser(boolean useThirdPartyUAParser) {
      this.useThirdPartyUAParser = useThirdPartyUAParser;
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
