package com.statsig;

import com.statsig.internal.NativeBinaryResolver;
import java.io.*;
import java.net.URL;
import java.util.Map;

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

    LIBRARY_LOADED = loadNativeLibraryFromResources();

    if (!LIBRARY_LOADED) {
      logNativeLibraryError();
    }
  }

  /** Statsig */
  public static native long statsigCreate(String sdkKey, long optionsRef, String statsigMetadata);

  public static native void statsigRelease(long statsigRef);

  public static native void statsigInitialize(long statsigRef, Runnable callback);

  public static native String statsigInitializeWithDetails(long statsigRef);

  public static native void statsigShutdown(long statsigRef, Runnable callback);

  public static native boolean statsigCheckGate(
      long statsigRef, long userRef, String gateName, CheckGateOptions options);

  public static native String statsigGetFeatureGate(
      long statsigRef, long userRef, String gateName, CheckGateOptions options);

  public static native String statsigGetFieldsNeededForGate(long statsigRef, String gateName);

  public static native void statsigLogGateExposure(long statsigRef, long userRef, String gateName);

  public static native String statsigGetLayer(
      long statsigRef, long userRef, String layerName, GetLayerOptions options);

  public static native void statsigManuallyLogLayerParamExposure(
      long statsigRef, long userRef, String layerName, String param);

  public static native void statsigIdentify(long statsigRef, long userRef);

  public static native String statsigGetFieldsNeededForLayer(long statsigRef, String layerName);

  public static native String statsigGetExperiment(
      long statsigRef, long userRef, String experimentName, GetExperimentOptions options);

  public static native void statsigLogExperimentExposure(
      long statsigRef, long userRef, String experimentName);

  public static native String statsigGetFieldsNeededForExperiment(
      long statsigRef, String experimentName);

  public static native String statsigGetDynamicConfig(
      long statsigRef, long userRef, String configName, GetDynamicConfigOptions options);

  public static native void statsigLogDynamicConfigExposure(
      long statsigRef, long userRef, String configName);

  public static native String statsigGetFieldsNeededForDynamicConfig(
      long statsigRef, String configName);

  public static native String statsigGetCMABRankedVariants(
      long statsigRef, long userRef, String cmabName);

  public static native void statsigLogCMABExposure(
      long statsigRef, long userRef, String cmabName, String ruleId);

  public static native String statsigGetClientInitResponse(
      long statsigRef, long userRef, ClientInitResponseOptions options);

  public static native void statsigLogEvent(
      long statsigRef, long userRef, String eventName, String value, Map<String, String> metadata);

  public static native void statsigLogEventWithLong(
      long statsigRef, long userRef, String eventName, long value, Map<String, String> metadata);

  public static native void statsigLogEventWithDouble(
      long statsigRef, long userRef, String eventName, double value, Map<String, String> metadata);

  public static native void statsigFlushEvents(long statsigRef, Runnable callback);

  public static native void statsigLogLayerParamExposure(
      long statsigRef, String layerJson, String param);

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

  /** Local Overrides */
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

  /** Remove Overrides */
  public static native void statsigRemoveGateOverride(long statsigRef, String gateName, String id);

  public static native void statsigRemoveDynamicConfigOverride(
      long statsigRef, String configName, String id);

  public static native void statsigRemoveExperimentOverride(
      long statsigRef, String experimentName, String id);

  public static native void statsigRemoveLayerOverride(
      long statsigRef, String layerName, String id);

  public static native void statsigRemoveAllOverrides(long statsigRef);

  /** StatsigUser */
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
      ProxyConfig proxyConfig,
      boolean enableIDLists,
      boolean waitForCountryLookupInit,
      boolean disableAllLogging,
      boolean waitForUserAgentInit,
      boolean disableNetwork,
      boolean disableUserCountry,
      boolean disableUserAgent,
      boolean fallbackToStatsigApi);

  public static native void statsigOptionsRelease(long optionsRef);

  /** [Internal] Library Loading */
  private static boolean loadNativeLibraryFromResources() {
    String osName = NativeBinaryResolver.osName;
    String arch = NativeBinaryResolver.normalizedArch;
    try {
      URL resource = NativeBinaryResolver.findLibraryResource();

      if (resource == null) {
        OutputLogger.logError(
            TAG, "Unable to find native library resource for OS: " + osName + " Arch: " + arch);
        return false;
      }

      OutputLogger.logInfo(TAG, "Loading native library: " + resource);
      String libPath = writeLibToTempFile(resource);

      if (libPath == null) {
        return false;
      }

      OutputLogger.logInfo(TAG, "Loaded native library: " + libPath);
      System.load(libPath);

      return true;
    } catch (UnsatisfiedLinkError e) {
      OutputLogger.logError(
          TAG,
          String.format(
              "Error: Failed to load native library for the specific OS and architecture. "
                  + "Operating System: %s, Architecture: %s. "
                  + "Please ensure that the necessary dependencies have been added to your project configuration.\n",
              osName, arch));
      OutputLogger.logError(TAG, e.getMessage());
      return false;
    }
  }

  private static String writeLibToTempFile(URL resource) {
    try (InputStream in = resource.openStream()) {
      if (in == null) {
        OutputLogger.logError(TAG, "Unable to open stream for resource: " + resource);
        return null;
      }

      File temp = File.createTempFile("statsig_java_lib", null);
      temp.deleteOnExit();

      try (OutputStream out = new FileOutputStream(temp)) {
        byte[] buffer = new byte[1024];
        int length;
        while ((length = in.read(buffer)) != -1) {
          out.write(buffer, 0, length);
        }
      }

      OutputLogger.logInfo(
          TAG,
          "Successfully created a temporary file for the native library at: "
              + temp.getAbsolutePath());
      return temp.getAbsolutePath();
    } catch (IOException e) {
      OutputLogger.logError(
          TAG, "I/O Error while writing the library to a temporary file: " + e.getMessage());
      return null;
    }
  }

  private static void logNativeLibraryError() {
    String osName = NativeBinaryResolver.osName;
    String arch = NativeBinaryResolver.normalizedArch;
    StringBuilder message =
        new StringBuilder("Ensure the correct native library is available for your platform.\n");
    String normalizedOsName = osName.toLowerCase().replace(" ", "");

    if (normalizedOsName.contains("macos")) {
      addDependency(message, "macOS", arch, "macos");
    } else if (normalizedOsName.contains("linux")) {
      addDependency(message, "Linux", arch, "linux-gnu", "amazonlinux2", "amazonlinux2023");
    } else if (normalizedOsName.contains("win")) {
      addDependency(message, "Windows", arch, "windows");
    } else {
      message.append(String.format("Unsupported OS: %s. Check your environment.\n", osName));
    }

    message.append("For further assistance, refer to the documentation or contact support.");
    OutputLogger.logError(TAG, message.toString());
  }

  private static void addDependency(
      StringBuilder message, String os, String arch, String... platforms) {
    message.append(
        String.format(
            "For %s with %s architecture, add the following to build.gradle:\n", os, arch));
    for (String platform : platforms) {
      message.append(
          String.format(
              "  implementation 'com.statsig:javacore:<version>:%s-%s'\n", platform, arch));
    }
  }
}
