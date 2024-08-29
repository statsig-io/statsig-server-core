package com.statsig;

import java.util.Map;

public class StatsigJNI {
    private static boolean libraryLoaded;

    public static boolean isLibraryLoaded() {
        return libraryLoaded;
    }

    static {
        try {
            System.setProperty("java.library.path", "lib");
            System.loadLibrary("statsig_ffi");
            libraryLoaded = true;
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Failed to load libstatsig_ffi: " + e.getMessage());
            libraryLoaded = false;
        }
    }

    /**
     * Statsig
     */
    public static native int statsigCreate(String sdkKey, int optionsRef);
    public static native void statsigRelease(int statsigRef);
    public static native void statsigInitialize(int statsigRef, Runnable callback);
    public static native void statsigShutdown(int statsigRef, Runnable callback);
    public static native boolean statsigCheckGate(int statsigRef, int userRef, String gateName);
    public static native String statsigGetFeatureGate(int statsigRef, int userRef, String gateName);
    public static native String statsigGetLayer(int statsigRef, int userRef, String layerName);
    public static native String statsigGetExperiment(int statsigRef, int userRef, String experimentName);
    public static native String statsigGetDynamicConfig(int statsigRef, int userRef, String configName);
    public static native String statsigGetClientInitResponse(int statsigRef, int userRef);
    public static native void statsigLogEvent(int statsigRef, int userRef, String eventName, String value, Map<String, String> metadata);
    public static native void statsigFlushEvents(int statsigRef, Runnable callback);

    /**
     * StatsigUser
     */
    public static native int statsigUserCreate(
            String userId,
            String customIdsJson,
            String email,
            String ip,
            String userAgent,
            String country,
            String locale,
            String appVersion,
            String customJson,
            String privateAttributesJson
    );
    public static native void statsigUserRelease(int userRef);

    /**
     * StatsigOptions
     */
    public static native int statsigOptionsCreate(
            String specsUrl,
            String logEventUrl,
            long specsSyncIntervalMs,
            long eventLoggingFlushIntervalMs,
            long eventLoggingMaxQueueSize,
            String environment
    );
    public static native void statsigOptionsRelease(int optionsRef);
}
