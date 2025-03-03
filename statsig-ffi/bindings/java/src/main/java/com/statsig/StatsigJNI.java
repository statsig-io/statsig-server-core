package com.statsig;

import java.io.*;
import java.net.URL;
import java.util.Map;

class StatsigJNI {
    private static final boolean libraryLoaded;
    private static final String TAG = "StatsigJNI";

    static boolean isLibraryLoaded() {
        return libraryLoaded;
    }

    static {
        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();

        OutputLogger.logInfo(TAG, "Detected OS: " + osName + " Arch: " + osArch);

        libraryLoaded = loadNativeLibrary(osName, osArch);

        if (!libraryLoaded) {
            logNativeLibraryError(osName, osArch);
        }
    }

    /**
     * Statsig
     */
    public static native String statsigCreate(String sdkKey, String optionsRef);

    public static native void statsigRelease(String statsigRef);

    public static native void statsigInitialize(String statsigRef, Runnable callback);

    public static native void statsigSequencedShutdownPrepare(String statsigRef, Runnable callback);

    public static native void statsigFinalizeShutdown(String statsigRef);

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
     */
    public static native String statsigOptionsCreate(
            String specsUrl,
            String logEventUrl,
            String idListsUrl,
            long specsSyncIntervalMs,
            long eventLoggingFlushIntervalMs,
            long eventLoggingMaxQueueSize,
            String environment,
            boolean enableIDLists,
            boolean disableAllLogging,
            long outputLoggerLevel,
            String serviceName,
            boolean enableUserAgentParsing,
            boolean enableCountryLookup);

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
                            "Operating System: %s, Architecture: %s. Please ensure that the necessary dependencies have been added to your project configuration.\n",
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
            resource = cl.getResource("native/libstatsig_ffi.dll");
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
            addDependency(message, "Linux", arch, "amazonlinux2", "amazonlinux2023");
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
