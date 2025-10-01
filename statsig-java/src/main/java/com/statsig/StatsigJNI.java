package com.statsig;

import com.statsig.internal.NativeBinaryResolver;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

class StatsigJNI {
  private static final boolean LIBRARY_LOADED;
  private static final String TAG = "StatsigJNI";

  static boolean isLibraryLoaded() {
    return LIBRARY_LOADED;
  }

  static {
    OutputLogger.logInfo(
        TAG,
        "Detected OS: "
            + NativeBinaryResolver.osName
            + " Arch: "
            + NativeBinaryResolver.normalizedArch);

    LIBRARY_LOADED = NativeBinaryResolver.load();
  }

  // ------------------------------------------------------------------------------------------------------- [Statsig]

  public static native long statsigCreate(String sdkKey, long optionsRef);

  public static native void statsigRelease(long statsigRef);

  public static native void updateStatsigMetadata(String metadata);

  public static native void statsigInitialize(long statsigRef, Runnable callback);

  public static native void statsigInitializeWithDetails(
      long statsigRef, CompletableFuture<String> future);

  public static native void statsigShutdown(long statsigRef, Runnable callback);

  // -----------------------------------------------------------------------------------------
  // [Statsig: Feature Gate]

  public static native boolean statsigCheckGate(
      long statsigRef, long userRef, String gateName, CheckGateOptions options);

  public static native String statsigGetFeatureGate(
      long statsigRef, long userRef, String gateName, CheckGateOptions options);

  public static native String statsigGetFieldsNeededForGate(long statsigRef, String gateName);

  public static native void statsigLogGateExposure(long statsigRef, long userRef, String gateName);

  // -------------------------------------------------------------------------------------------
  // [Statsig: Experiment]

  public static native String statsigGetExperiment(
      long statsigRef, long userRef, String experimentName, GetExperimentOptions options);

  public static native void statsigLogExperimentExposure(
      long statsigRef, long userRef, String experimentName);

  public static native String statsigGetFieldsNeededForExperiment(
      long statsigRef, String experimentName);

  // ----------------------------------------------------------------------------------------
  // [Statsig: DynamicConfig]

  public static native String statsigGetDynamicConfig(
      long statsigRef, long userRef, String configName, GetDynamicConfigOptions options);

  public static native void statsigLogDynamicConfigExposure(
      long statsigRef, long userRef, String configName);

  public static native String statsigGetFieldsNeededForDynamicConfig(
      long statsigRef, String configName);

  // ------------------------------------------------------------------------------------------------ [Statsig: Layer]

  public static native String statsigGetLayer(
      long statsigRef, long userRef, String layerName, GetLayerOptions options);

  public static native String statsigGetPrompt(
      long statsigRef, long userRef, String promptName, GetLayerOptions options);

  public static native void statsigManuallyLogLayerParamExposure(
      long statsigRef, long userRef, String layerName, String param);

  public static native String statsigGetFieldsNeededForLayer(long statsigRef, String layerName);

  public static native void statsigLogLayerParamExposure(
      long statsigRef, String layerJson, String param);

  // ----------------------------------------------------------------------------------------
  // [Statsig: Event Logging]

  public static native void statsigLogEvent(
      long statsigRef, long userRef, String eventName, String value, Map<String, String> metadata);

  public static native void statsigLogEventWithLong(
      long statsigRef, long userRef, String eventName, long value, Map<String, String> metadata);

  public static native void statsigLogEventWithDouble(
      long statsigRef, long userRef, String eventName, double value, Map<String, String> metadata);

  public static native void statsigFlushEvents(long statsigRef, Runnable callback);

  // ------------------------------------------------------------------------------------------
  // [Statsig: Param Store]

  public static native String statsigGetParameterStore(long statsigRef, String parameterStoreName);

  public static native String statsigGetStringParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      String defaultValue);

  public static native boolean statsigGetBooleanParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      boolean defaultValue);

  public static native double statsigGetFloatParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      double defaultValue);

  public static native long statsigGetIntegerParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      long defaultValue);

  public static native String statsigGetObjectParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      String defaultValue);

  public static native String statsigGetArrayParameterFromParameterStore(
      long statsigRef,
      long userRef,
      String parameterStoreName,
      String parameterName,
      String defaultValue);

  // --------------------------------------------------------------------------------------
  // [Statsig: Local Overrides]

  public static native void statsigOverrideGate(
      long statsigRef, String gateName, String id, boolean overrideVal);

  public static native void statsigOverrideDynamicConfig(
      long statsigRef, String configName, String id, Map<String, Object> overrideVal);

  public static native void statsigOverrideLayer(
      long statsigRef, String layerName, String id, Map<String, Object> overrideVal);

  public static native void statsigOverrideExperiment(
      long statsigRef, String experimentName, String id, Map<String, Object> overrideVal);

  public static native void statsigOverrideExperimentByGroupName(
      long statsigRef, String experimentName, String id, String groupName);

  public static native void statsigRemoveGateOverride(long statsigRef, String gateName, String id);

  public static native void statsigRemoveDynamicConfigOverride(
      long statsigRef, String configName, String id);

  public static native void statsigRemoveExperimentOverride(
      long statsigRef, String experimentName, String id);

  public static native void statsigRemoveLayerOverride(
      long statsigRef, String layerName, String id);

  public static native void statsigRemoveAllOverrides(long statsigRef);

  // ------------------------------------------------------------------------------------------------- [Statsig: Misc]

  public static native String statsigGetClientInitResponse(
      long statsigRef, long userRef, ClientInitResponseOptions options);

  public static native String statsigGetCMABRankedVariants(
      long statsigRef, long userRef, String cmabName);

  public static native void statsigLogCMABExposure(
      long statsigRef, long userRef, String cmabName, String ruleId);

  public static native void statsigIdentify(long statsigRef, long userRef);

  // --------------------------------------------------------------------------------------------------- [StatsigUser]

  public static native long statsigUserCreate(
      String userId,
      String customIdsJson,
      String email,
      String ip,
      String userAgent,
      String country,
      String locale,
      String appVersion,
      String customJson,
      String privateAttributesJson);

  public static native void statsigUserRelease(long userRef);

  // ------------------------------------------------------------------------------------------------ [StatsigOptions]

  /**
   * StatsigOptions
   *
   * <p>WARNING: The order of parameters in this method **MUST MATCH EXACTLY** with the
   * corresponding Rust implementation in `statsig_options_jni.rs`. Any mismatch will cause
   * incorrect values to be passed across the JNI boundary.
   */
  public static native long statsigOptionsCreate(
      String specsUrl,
      String logEventUrl,
      String idListsUrl,
      long idListsSyncIntervalMs,
      long specsSyncIntervalMs,
      long eventLoggingFlushIntervalMs,
      long eventLoggingMaxQueueSize,
      long eventLoggingMaxPendingBatchQueueSize,
      long initTimeoutMs,
      String environment,
      long outputLoggerLevel,
      String serviceName,
      ObservabilityClient observabilityClient,
      DataStore dataStore,
      OutputLoggerProvider outputLoggerProvider,
      ProxyConfig proxyConfig,
      boolean enableIDLists,
      boolean waitForCountryLookupInit,
      boolean disableAllLogging,
      boolean waitForUserAgentInit,
      boolean disableNetwork,
      boolean disableUserCountry,
      boolean fallbackToStatsigApi,
      boolean useThirdPartyUAParser);

  public static native void statsigOptionsRelease(long optionsRef);
}
