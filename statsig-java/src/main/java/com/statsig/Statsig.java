package com.statsig;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.statsig.internal.JacksonUtil;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;

public class Statsig {

  // Shared Instance
  private static volatile Statsig sharedInstance = null;

  public static Statsig shared() {
    if (!hasShared()) {
      System.err.println(
          "[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance");
      return createErrorStatsigInstance();
    }

    return sharedInstance;
  }

  public static boolean hasShared() {
    return sharedInstance != null;
  }

  public static Statsig newShared(String sdkKey, StatsigOptions options) {
    if (hasShared()) {
      System.err.println(
          "[Statsig] Shared instance has been created, call removeShared() if you want to create another one. "
              + "Returning an invalid instance");
      return createErrorStatsigInstance();
    }
    sharedInstance = new Statsig(sdkKey, options);

    return sharedInstance;
  }

  public static Statsig newShared(String sdkKey) {
    if (hasShared()) {
      System.err.println(
          "[Statsig] Shared instance has been created, call removeSharedInstance() if you want to create another one. "
              + "Returning an invalid instance");
      return createErrorStatsigInstance();
    }
    sharedInstance = new Statsig(sdkKey);

    return sharedInstance;
  }

  public static void removeSharedInstance() {
    sharedInstance = null;
  }

  private static final ObjectMapper MAPPER = JacksonUtil.getObjectMapper();

  private volatile long ref;
  private final ScheduledExecutorService scheduler = Executors.newScheduledThreadPool(1);

  /**
   * Instantiates a new Statsig instance that connects to Statsig Service.
   *
   * <p>It is recommended to create a single instance for the entire application's lifecycle. In
   * rare situations where the application requires feature evaluation from different Statsig
   * projects or environments, you may instantiate multiple instances. However, these should be
   * maintained throughout the application's lifecycle, rather than being created for each request
   * or thread.
   *
   * @param sdkKey secret key to connect to Statsig Service
   * @param options a customized instance of StatsigOptions that configures the behavior of the
   *     Statsig instance.
   */
  public Statsig(String sdkKey, StatsigOptions options) {
    StatsigJNI.updateStatsigMetadata(StatsigMetadata.getSerializedCopy());
    this.ref = StatsigJNI.statsigCreate(sdkKey, options.getRef());

    ResourceCleaner.register(
        this,
        () -> {
          if (ref != 0) {
            StatsigJNI.statsigRelease(ref);
            ref = 0;
          }
        });
  }

  public Statsig(String sdkKey) {
    StatsigJNI.updateStatsigMetadata(StatsigMetadata.getSerializedCopy());
    this.ref = StatsigJNI.statsigCreate(sdkKey, 0);

    ResourceCleaner.register(
        this,
        () -> {
          if (ref != 0) {
            StatsigJNI.statsigRelease(ref);
            ref = 0;
          }
        });
  }

  public long getRef() {
    return ref;
  }

  public CompletableFuture<Void> initialize() {
    CompletableFuture<Void> future = new CompletableFuture<>();
    Runnable callback =
        () -> {
          // Complete the future when the native operation is done
          future.complete(null);
        };

    StatsigJNI.statsigInitialize(ref, callback);
    return future;
  }

  public CompletableFuture<Void> shutdown() {
    CompletableFuture<Void> future = new CompletableFuture<>();
    Runnable callback =
        () -> {
          // Complete the future when the native operation is done
          future.complete(null);
        };

    StatsigJNI.statsigShutdown(ref, callback);
    return future;
  }

  public boolean checkGate(StatsigUser user, String gateName) {
    return StatsigJNI.statsigCheckGate(ref, user.getRef(), gateName, null);
  }

  public boolean checkGate(StatsigUser user, String gateName, CheckGateOptions options) {
    return StatsigJNI.statsigCheckGate(ref, user.getRef(), gateName, options);
  }

  public void manuallyLogGateExposure(StatsigUser user, String gateName) {
    StatsigJNI.statsigLogGateExposure(ref, user.getRef(), gateName);
  }

  public String[] getFieldsNeededForGate(String gateName) {
    String resultJSON = StatsigJNI.statsigGetFieldsNeededForGate(ref, gateName);
    return JacksonUtil.fromJsonToStringArray(resultJSON);
  }

  public Experiment getExperiment(StatsigUser user, String experimentName) {
    String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, null);
    return Experiment.fromJson(experJson);
  }

  public Experiment getExperiment(
      StatsigUser user, String experimentName, GetExperimentOptions options) {
    String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, options);
    return Experiment.fromJson(experJson);
  }

  public void manuallyLogExperimentExposure(StatsigUser user, String experimentName) {
    StatsigJNI.statsigLogExperimentExposure(ref, user.getRef(), experimentName);
  }

  public String[] getFieldsNeededForExperiment(String experimentName) {
    String resultJSON = StatsigJNI.statsigGetFieldsNeededForExperiment(ref, experimentName);
    return JacksonUtil.fromJsonToStringArray(resultJSON);
  }

  public DynamicConfig getDynamicConfig(StatsigUser user, String configName) {
    String configJson = StatsigJNI.statsigGetDynamicConfig(ref, user.getRef(), configName, null);
    return JacksonUtil.fromJsonWithRawJson(configJson, DynamicConfig.class);
  }

  public DynamicConfig getDynamicConfig(
      StatsigUser user, String configName, GetDynamicConfigOptions options) {
    String configJson = StatsigJNI.statsigGetDynamicConfig(ref, user.getRef(), configName, options);
    return JacksonUtil.fromJsonWithRawJson(configJson, DynamicConfig.class);
  }

  public void manuallyLogDynamicConfigExposure(StatsigUser user, String configName) {
    StatsigJNI.statsigLogDynamicConfigExposure(ref, user.getRef(), configName);
  }

  public String[] getFieldsNeededForDynamicConfig(String configName) {
    String resultJSON = StatsigJNI.statsigGetFieldsNeededForDynamicConfig(ref, configName);
    return JacksonUtil.fromJsonToStringArray(resultJSON);
  }

  public Layer getLayer(StatsigUser user, String layerName) {
    String layerJson = StatsigJNI.statsigGetLayer(ref, user.getRef(), layerName, null);
    Layer layer = JacksonUtil.fromJsonWithRawJson(layerJson, Layer.class);
    if (layer != null) {
      // Set the Statsig reference in the Layer instance
      layer.setStatsigInstance(this);
    }
    return layer;
  }

  public Layer getLayer(StatsigUser user, String layerName, GetLayerOptions options) {
    String layerJson = StatsigJNI.statsigGetLayer(ref, user.getRef(), layerName, options);
    Layer layer = JacksonUtil.fromJsonWithRawJson(layerJson, Layer.class);
    if (layer != null) {
      // Set the Statsig reference in the Layer instance
      layer.setStatsigInstance(this);
      layer.setDisableExposureLogging(options != null && options.disableExposureLogging);
    }
    return layer;
  }

  public void manuallyLogLayerParamExposure(StatsigUser user, String layerName, String param) {
    StatsigJNI.statsigManuallyLogLayerParamExposure(ref, user.getRef(), layerName, param);
  }

  public String[] getFieldsNeededForLayer(String layerName) {
    String resultJSON = StatsigJNI.statsigGetFieldsNeededForLayer(ref, layerName);
    return JacksonUtil.fromJsonToStringArray(resultJSON);
  }

  public FeatureGate getFeatureGate(StatsigUser user, String gateName) {
    String gateJson = StatsigJNI.statsigGetFeatureGate(ref, user.getRef(), gateName, null);
    return JacksonUtil.fromJsonWithRawJson(gateJson, FeatureGate.class);
  }

  public FeatureGate getFeatureGate(StatsigUser user, String gateName, CheckGateOptions options) {
    String gateJson = StatsigJNI.statsigGetFeatureGate(ref, user.getRef(), gateName, options);
    return JacksonUtil.fromJsonWithRawJson(gateJson, FeatureGate.class);
  }

  public CompletableFuture<InitializeDetails> initializeWithDetails() {
    CompletableFuture<String> future = new CompletableFuture<>();

    StatsigJNI.statsigInitializeWithDetails(ref, future);

    return future.thenApply(json -> JacksonUtil.fromJsonWithRawJson(json, InitializeDetails.class));
  }

  public CMABRankedVariant[] getCMABRankedVariants(StatsigUser user, String cmabName) {
    String cmabJson = StatsigJNI.statsigGetCMABRankedVariants(ref, user.getRef(), cmabName);
    CMABRankedVariant[] result = JacksonUtil.fromJson(cmabJson, CMABRankedVariant[].class);

    return result != null ? result : new CMABRankedVariant[0];
  }

  public void logCMABExposure(StatsigUser user, String cmabName, String ruleId) {
    StatsigJNI.statsigLogCMABExposure(ref, user.getRef(), cmabName, ruleId);
  }

  public ParameterStore getParameterStore(StatsigUser user, String parameterStoreName) {
    String storeJson = StatsigJNI.statsigGetParameterStore(ref, parameterStoreName);
    ParameterStore store = JacksonUtil.fromJson(storeJson, ParameterStore.class);
    if (store != null) {
      // Set the Statsig reference in the Layer instance
      store.setStatsigInstance(this);
      store.setUser(user);
    }
    return store;
  }

  public String getStringFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, String defaultValue) {
    return StatsigJNI.statsigGetStringParameterFromParameterStore(
        ref, user.getRef(), parameterStoreName, parameterName, defaultValue);
  }

  public boolean getBooleanFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, boolean defaultValue) {
    return StatsigJNI.statsigGetBooleanParameterFromParameterStore(
        ref, user.getRef(), parameterStoreName, parameterName, defaultValue);
  }

  public double getDoubleFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, double defaultValue) {
    return StatsigJNI.statsigGetFloatParameterFromParameterStore(
        ref, user.getRef(), parameterStoreName, parameterName, defaultValue);
  }

  public long getLongFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, long defaultValue) {
    return StatsigJNI.statsigGetIntegerParameterFromParameterStore(
        ref, user.getRef(), parameterStoreName, parameterName, defaultValue);
  }

  public int getIntFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, int defaultValue) {
    return (int)
        StatsigJNI.statsigGetIntegerParameterFromParameterStore(
            ref, user.getRef(), parameterStoreName, parameterName, defaultValue);
  }

  public Map<String, Object> getMapFromParameterStore(
      StatsigUser user,
      String parameterStoreName,
      String parameterName,
      Map<String, Object> defaultValue) {
    String defaultValueJSON = JacksonUtil.toJson(defaultValue);

    String result =
        StatsigJNI.statsigGetObjectParameterFromParameterStore(
            ref, user.getRef(), parameterStoreName, parameterName, defaultValueJSON);
    if (result == null) {
      return defaultValue;
    }

    Map<String, Object> map =
        JacksonUtil.fromJson(result, new TypeReference<Map<String, Object>>() {});
    return map != null ? map : defaultValue;
  }

  public Object[] getArrayFromParameterStore(
      StatsigUser user, String parameterStoreName, String parameterName, Object[] defaultValue) {
    String defaultValueJSON = JacksonUtil.toJson(defaultValue);

    String result =
        StatsigJNI.statsigGetArrayParameterFromParameterStore(
            ref, user.getRef(), parameterStoreName, parameterName, defaultValueJSON);

    Object[] array = JacksonUtil.fromJson(result, Object[].class);
    return array != null ? array : defaultValue;
  }

  public void logEvent(StatsigUser user, String eventName) {
    StatsigJNI.statsigLogEvent(ref, user.getRef(), eventName, null, null);
  }

  public void logEvent(
      StatsigUser user, String eventName, String value, Map<String, String> metadata) {
    StatsigJNI.statsigLogEvent(ref, user.getRef(), eventName, value, metadata);
  }

  public void logEvent(StatsigUser user, String eventName, String value) {
    StatsigJNI.statsigLogEvent(ref, user.getRef(), eventName, value, null);
  }

  public void logEvent(
      StatsigUser user, String eventName, double value, Map<String, String> metadata) {
    StatsigJNI.statsigLogEventWithDouble(ref, user.getRef(), eventName, value, metadata);
  }

  public void logEvent(StatsigUser user, String eventName, double value) {
    StatsigJNI.statsigLogEventWithDouble(ref, user.getRef(), eventName, value, null);
  }

  public CompletableFuture<Void> flushEvents() {
    CompletableFuture<Void> future = new CompletableFuture<>();
    Runnable callback =
        () -> {
          future.complete(null);
        };

    StatsigJNI.statsigFlushEvents(ref, callback);
    return future;
  }

  public String getClientInitializeResponse(StatsigUser user) {
    // if no gcir option passed in, will default to djb2
    return StatsigJNI.statsigGetClientInitResponse(ref, user.getRef(), null);
  }

  public String getClientInitializeResponse(StatsigUser user, ClientInitResponseOptions options) {
    return StatsigJNI.statsigGetClientInitResponse(ref, user.getRef(), options);
  }

  /**
   * Overrides a gate with the specified value.
   *
   * @param gateName The name of the gate to override
   * @param gateValue The value to override the gate with
   */
  public void overrideGate(String gateName, boolean gateValue) {
    StatsigJNI.statsigOverrideGate(ref, gateName, null, gateValue);
  }

  /**
   * Overrides a gate with the specified value for a specific ID.
   *
   * @param gateName The name of the gate to override
   * @param id The ID to override the gate for
   * @param gateValue The value to override the gate with
   */
  public void overrideGate(String gateName, boolean gateValue, String id) {
    StatsigJNI.statsigOverrideGate(ref, gateName, id, gateValue);
  }

  /**
   * Overrides an experiment with the specified value.
   *
   * @param experimentName The name of the experiment to override
   * @param experimentValue The value to override the experiment with
   */
  public void overrideExperiment(String experimentName, Map<String, Object> experimentValue) {
    StatsigJNI.statsigOverrideExperiment(ref, experimentName, null, experimentValue);
  }

  /**
   * Overrides an experiment with the specified value for a specific ID.
   *
   * @param experimentName The name of the experiment to override
   * @param id The ID to override the experiment for
   * @param experimentValue The value to override the experiment with
   */
  public void overrideExperiment(
      String experimentName, Map<String, Object> experimentValue, String id) {
    StatsigJNI.statsigOverrideExperiment(ref, experimentName, id, experimentValue);
  }

  /**
   * Overrides a dynamic config with the specified value.
   *
   * @param dynamicConfigName The name of the dynamic config to override
   * @param dynamicConfigValue The value to override the dynamic config with
   */
  public void overrideDynamicConfig(
      String dynamicConfigName, Map<String, Object> dynamicConfigValue) {
    StatsigJNI.statsigOverrideDynamicConfig(ref, dynamicConfigName, null, dynamicConfigValue);
  }

  /**
   * Overrides a dynamic config with the specified value for a specific ID.
   *
   * @param dynamicConfigName The name of the dynamic config to override
   * @param id The ID to override the dynamic config for
   * @param dynamicConfigValue The value to override the dynamic config with
   */
  public void overrideDynamicConfig(
      String dynamicConfigName, Map<String, Object> dynamicConfigValue, String id) {
    StatsigJNI.statsigOverrideDynamicConfig(ref, dynamicConfigName, id, dynamicConfigValue);
  }

  /**
   * Overrides a layer with the specified value.
   *
   * @param layerName The name of the layer to override
   * @param layerValue The value to override the layer with
   */
  public void overrideLayer(String layerName, Map<String, Object> layerValue) {
    StatsigJNI.statsigOverrideLayer(ref, layerName, null, layerValue);
  }

  /**
   * Overrides a layer with the specified value for a specific ID.
   *
   * @param layerName The name of the layer to override
   * @param id The ID to override the layer for
   * @param layerValue The value to override the layer with
   */
  public void overrideLayer(String layerName, Map<String, Object> layerValue, String id) {
    StatsigJNI.statsigOverrideLayer(ref, layerName, id, layerValue);
  }

  /**
   * Overrides an experiment with the specified group name.
   *
   * @param experimentName The name of the experiment to override
   * @param groupName The group name to override the experiment with
   */
  public void overrideExperimentByGroupName(String experimentName, String groupName) {
    StatsigJNI.statsigOverrideExperimentByGroupName(ref, experimentName, null, groupName);
  }

  /**
   * Overrides an experiment with the specified group name for a specific ID.
   *
   * @param experimentName The name of the experiment to override
   * @param id The ID to override the experiment for
   * @param groupName The group name to override the experiment with
   */
  public void overrideExperimentByGroupName(String experimentName, String groupName, String id) {
    StatsigJNI.statsigOverrideExperimentByGroupName(ref, experimentName, id, groupName);
  }

  /**
   * Removes all overrides for the specified gate.
   *
   * @param gateName The name of the gate to remove overrides for
   */
  public void removeGateOverride(String gateName) {
    StatsigJNI.statsigRemoveGateOverride(ref, gateName, null);
  }

  /**
   * Removes overrides for the specified gate and ID.
   *
   * @param gateName The name of the gate to remove overrides for
   * @param id The ID to remove overrides for
   */
  public void removeGateOverride(String gateName, String id) {
    StatsigJNI.statsigRemoveGateOverride(ref, gateName, id);
  }

  /**
   * Removes all overrides for the specified dynamic config.
   *
   * @param configName The name of the dynamic config to remove overrides for
   */
  public void removeDynamicConfigOverride(String configName) {
    StatsigJNI.statsigRemoveDynamicConfigOverride(ref, configName, null);
  }

  /**
   * Removes overrides for the specified dynamic config and ID.
   *
   * @param configName The name of the dynamic config to remove overrides for
   * @param id The ID to remove overrides for
   */
  public void removeDynamicConfigOverride(String configName, String id) {
    StatsigJNI.statsigRemoveDynamicConfigOverride(ref, configName, id);
  }

  /**
   * Removes all overrides for the specified experiment.
   *
   * @param experimentName The name of the experiment to remove overrides for
   */
  public void removeExperimentOverride(String experimentName) {
    StatsigJNI.statsigRemoveExperimentOverride(ref, experimentName, null);
  }

  /**
   * Removes overrides for the specified experiment and ID.
   *
   * @param experimentName The name of the experiment to remove overrides for
   * @param id The ID to remove overrides for
   */
  public void removeExperimentOverride(String experimentName, String id) {
    StatsigJNI.statsigRemoveExperimentOverride(ref, experimentName, id);
  }

  /**
   * Removes all overrides for the specified layer.
   *
   * @param layerName The name of the layer to remove overrides for
   */
  public void removeLayerOverride(String layerName) {
    StatsigJNI.statsigRemoveLayerOverride(ref, layerName, null);
  }

  /**
   * Removes overrides for the specified layer and ID.
   *
   * @param layerName The name of the layer to remove overrides for
   * @param id The ID to remove overrides for
   */
  public void removeLayerOverride(String layerName, String id) {
    StatsigJNI.statsigRemoveLayerOverride(ref, layerName, id);
  }

  /** Removes all overrides for all gates, dynamic configs, experiments, and layers. */
  public void removeAllOverrides() {
    StatsigJNI.statsigRemoveAllOverrides(ref);
  }

  public void identify(StatsigUser user) {
    StatsigJNI.statsigIdentify(ref, user.getRef());
  }

  void logLayerParamExposure(String layerJson, String param) {
    StatsigJNI.statsigLogLayerParamExposure(ref, layerJson, param);
  }

  private static Statsig createErrorStatsigInstance() {
    return new Statsig("INVALID_SECRET_KEY");
  }
}
