package com.statsig;

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
        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();

        OutputLogger.logInfo(TAG, "Detected OS: " + osName + " Arch: " + osArch);

        LIBRARY_LOADED = loadNativeLibrary(osName, osArch);

        if (!LIBRARY_LOADED) {
            logNativeLibraryError(osName, osArch);
        }
    }

    /**
     * Statsig
     */
    public static native String statsigCreate(String sdkKey, String optionsRef, String statsigMetadata);

    public static native void statsigRelease(String statsigRef);

    public static native void statsigInitialize(String statsigRef, Runnable callback);

    public static native void statsigShutdown(String statsigRef, Runnable callback);

    public static native boolean statsigCheckGate(String statsigRef, String userRef, String gateName,
            CheckGateOptions options);

    public static native String statsigGetFeatureGate(String statsigRef, String userRef, String gateName,
            CheckGateOptions options);

    public static native String statsigGetFieldsNeededForGate(String statsigRef, String gateName);

    public static native void statsigLogGateExposure(String statsigRef, String userRef, String gateName);

    public static native String statsigGetLayer(String statsigRef, String userRef, String layerName,
            GetLayerOptions options);

    public static native void statsigManuallyLogLayerParamExposure(String statsigRef, String userRef, String layerName,
            String param);

    public static native void statsigIdentify(String statsigRef, String userRef);

    public static native String statsigGetFieldsNeededForLayer(String statsigRef, String layerName);

    public static native String statsigGetExperiment(String statsigRef, String userRef, String experimentName,
            GetExperimentOptions options);

    public static native void statsigLogExperimentExposure(String statsigRef, String userRef, String experimentName);

    public static native String statsigGetFieldsNeededForExperiment(String statsigRef, String experimentName);

    public static native String statsigGetDynamicConfig(String statsigRef, String userRef, String configName,
            GetDynamicConfigOptions options);

    public static native void statsigLogDynamicConfigExposure(String statsigRef, String userRef, String configName);

    public static native String statsigGetFieldsNeededForDynamicConfig(String statsigRef, String configName);

    public static native String statsigGetCMABRankedVariants(String statsigRef, String userRef, String cmabName);

    public static native void statsigLogCMABExposure(String statsigRef, String userRef, String cmabName, String ruleId);

    public static native String statsigGetClientInitResponse(String statsigRef, String userRef,
            ClientInitResponseOptions options);

    public static native void statsigLogEvent(String statsigRef, String userRef, String eventName, String value,
            Map<String, String> metadata);

    public static native void statsigLogEventWithLong(String statsigRef, String userRef, String eventName, long value,
            Map<String, String> metadata);

    public static native void statsigLogEventWithDouble(String statsigRef, String userRef, String eventName,
            double value, Map<String, String> metadata);

    public static native void statsigFlushEvents(String statsigRef, Runnable callback);

    public static native void statsigLogLayerParamExposure(String statsigRef, String layerJson, String param);

    public static native String statsigGetParameterStore(String statsigRef, String parameterStoreName);

    public static native String statsigGetStringParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, String defaultValue);

    public static native boolean statsigGetBooleanParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, boolean defaultValue);

    public static native double statsigGetFloatParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, double defaultValue);

    public static native long statsigGetIntegerParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, long defaultValue);

    public static native String statsigGetObjectParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, String defaultValue);

    public static native String statsigGetArrayParameterFromParameterStore(String statsigRef, String userRef,
            String parameterStoreName, String parameterName, String defaultValue);

    /**
     * Local Overrides
     */
    public static native void statsigOverrideGate(String statsigRef, String gateName, String id, boolean overrideVal);

    public static native void statsigOverrideDynamicConfig(String statsigRef, String configName, String id, Map<String, Object> overrideVal);

    public static native void statsigOverrideLayer(String statsigRef, String layerName, String id, Map<String, Object> overrideVal);

    public static native void statsigOverrideExperiment(String statsigRef, String experimentName, String id, Map<String, Object> overrideVal);

    public static native void statsigOverrideExperimentByGroupName(String statsigRef, String experimentName, String id, String groupName);
    
    /**
     * Remove Overrides
     */
    public static native void statsigRemoveGateOverride(String statsigRef, String gateName, String id);

    public static native void statsigRemoveDynamicConfigOverride(String statsigRef, String configName, String id);

    public static native void statsigRemoveExperimentOverride(String statsigRef, String experimentName, String id);

    public static native void statsigRemoveLayerOverride(String statsigRef, String layerName, String id);
    
    public static native void statsigRemoveAllOverrides(String statsigRef);

    /**
     * StatsigUser
     */
    public static native String statsigUserCreate(
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

    public static native void statsigUserRelease(String userRef);

    /**
     * StatsigOptions
     *
     * WARNING: The order of parameters in this method **MUST MATCH EXACTLY**
     * with the corresponding Rust implementation in `statsig_options_jni.rs`.
     * Any mismatch will cause incorrect values to be passed across the JNI boundary.
     */
    public static native String statsigOptionsCreate(
            String specsUrl,
            String logEventUrl,
            String idListsUrl,
            long idListsSyncIntervalMs,
            long specsSyncIntervalMs,
            long eventLoggingFlushIntervalMs,
            long eventLoggingMaxQueueSize,
            long initTimeoutMs,
            String environment,
            long outputLoggerLevel,
            String serviceName,
            ObservabilityClient observabilityClient,
            boolean enableIDLists,
            boolean waitForCountryLookupInit,
            boolean disableAllLogging,
            boolean waitForUserAgentInit,
            boolean disableNetwork,
            boolean disableUserCountry,
            boolean disableUserAgent,
            boolean fallbackToStatsigApi
            );

    public static native void statsigOptionsRelease(String optionsRef);

    /**
     * [Internal] Library Loading
     */

    private static boolean loadNativeLibrary(String osName, String osArch) {
        try {
            URL resource = findLibraryResource(osName, osArch);

            if (resource == null) {
                OutputLogger.logError(
                        TAG,
                        "Unable to find native library resource for OS: " + osName + " Arch: " + osArch);
                return false;
            }

            OutputLogger.logInfo(
                    TAG,
                    "Loading native library: " + resource);
            String libPath = writeLibToTempFile(resource);

            if (libPath == null) {
                return false;
            }

            OutputLogger.logInfo(
                    TAG,
                    "Loaded native library: " + libPath);
            System.load(libPath);

            return true;
        } catch (UnsatisfiedLinkError e) {
            OutputLogger.logError(
                    TAG,
                    String.format("Error: Failed to load native library for the specific OS and architecture. " +
                            "Operating System: %s, Architecture: %s. " +
                            "Please ensure that the necessary dependencies have been added to your project configuration.\n",
                            osName, osArch));
            OutputLogger.logError(TAG, e.getMessage());
            return false;
        }
    }

    private static String writeLibToTempFile(URL resource) {
        try {
            InputStream stream = resource.openStream();

            if (stream == null) {
                OutputLogger.logError(TAG, "Unable to open stream for resource: " + resource);
                return null;
            }

            File temp = File.createTempFile("statsig_ffi_lib", null);
            temp.deleteOnExit();

            try (stream; OutputStream out = new FileOutputStream(temp)) {
                byte[] buffer = new byte[1024];
                int length = 0;
                while ((length = stream.read(buffer)) != -1) {
                    out.write(buffer, 0, length);
                }
            }

            OutputLogger.logInfo(TAG,
                    "Successfully created a temporary file for the native library at: " + temp.getAbsolutePath());
            return temp.getAbsolutePath();
        } catch (IOException e) {
            OutputLogger.logError(TAG,
                    "I/O Error while writing the library to a temporary file: " + e.getMessage());
            return null;
        }
    }

    private static URL findLibraryResource(String osName, String osArch) {
        ClassLoader cl = StatsigJNI.class.getClassLoader();
        URL resource = null;

        if (osName.contains("win")) {
            // NOTE: windows native lib NOT start with lib
            resource = cl.getResource("native/statsig_ffi.dll");
        } else if (osName.contains("mac")) {
            resource = cl.getResource("native/libstatsig_ffi.dylib");
        } else if (osName.contains("linux")) {
            resource = cl.getResource("native/libstatsig_ffi.so");
        }

        return resource;
    }

    private static void logNativeLibraryError(String osName, String osArch) {
        StringBuilder message = new StringBuilder(
                "Ensure the correct native library is available for your platform.\n");
        String normalizedOsName = osName.toLowerCase().replace(" ", "");

        String arch = normalizeArch(osArch);

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

    private static String normalizeArch(String osArch) {
        osArch = osArch.toLowerCase();

        if (osArch.contains("aarch64") || osArch.contains("arm64")) {
            return "arm64";
        } else if (osArch.contains("x86_64") || osArch.contains("amd64")) {
            return "x86_64";
        } else if (osArch.contains("i686")) {
            return "i686";
        } else {
            return osArch;
        }
    }

    private static void addDependency(StringBuilder message, String os, String arch, String... platforms) {
        message.append(String.format("For %s with %s architecture, add the following to build.gradle:\n", os, arch));
        for (String platform : platforms) {
            message.append(String.format("  implementation 'com.statsig:javacore:<version>:%s-%s'\n", platform, arch));
        }
    }

}
