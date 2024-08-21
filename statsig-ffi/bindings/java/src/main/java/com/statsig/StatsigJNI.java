package com.statsig;

public class StatsigJNI {
    private static boolean libraryLoaded;

    public static boolean isLibraryLoaded() {
        return libraryLoaded;
    }

    static {
        try {
            System.setProperty("java.library.path", "lib/native");
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
    public static native long statsigCreate(String sdkKey, long optionsRef);
    public static native void statsigRelease(long statsigRef);
    public static native void statsigInitialize(long statsigRef, Runnable callback);
    public static native void statsigShutdown(long statsigRef, Runnable callback);
    public static native boolean statsigCheckGate(long statsigRef, long userRef, String gateName);
    public static native String statsigGetClientInitResponse(long statsigRef, long userRef);

    /**
     * StatsigUser
     */
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
            String privateAttributesJson
    );
    public static native void statsigUserRelease(long userRef);

    /**
     * StatsigOptions
     */
    public static native long statsigOptionsCreate(
            String specsUrl,
            String logEventUrl,
            long specsSyncIntervalMs,
            long eventLoggingFlushIntervalMs,
            long eventLoggingMaxQueueSize,
            String environment
    );
    public static native void statsigOptionsRelease(long optionsRef);
}
