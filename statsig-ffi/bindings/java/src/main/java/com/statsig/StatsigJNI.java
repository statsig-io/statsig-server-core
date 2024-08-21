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
    public static native int statsigCreate(String sdkKey, int optionsRef);
    public static native void statsigRelease(int statsigRef);
    public static native void statsigInitialize(int statsigRef, Runnable callback);
    public static native void statsigShutdown(int statsigRef, Runnable callback);
    public static native boolean statsigCheckGate(int statsigRef, int userRef, String gateName);
    public static native String statsigGetClientInitResponse(int statsigRef, int userRef);

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
