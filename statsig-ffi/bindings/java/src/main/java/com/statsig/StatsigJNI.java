package com.statsig;

import java.io.*;
import java.net.URL;
import java.util.Map;

public class StatsigJNI {
    private static final boolean libraryLoaded;

    public static boolean isLibraryLoaded() {
        return libraryLoaded;
    }

    static {
        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();

        StatsigLogger.logInfo("Detected OS: " + osName + " Arch: " + osArch);

        libraryLoaded = loadNativeLibrary(osName, osArch);

        if (!libraryLoaded) {
            StatsigLogger.logNativeLibraryError(osName, osArch);
        }
    }

    private static boolean loadNativeLibrary(String osName, String osArch) {
        try {
            URL resource = findLibraryResource(osName, osArch);

            if (resource == null) {
                StatsigLogger.logError("Unable to find native library resource for OS: " + osName + " Arch: " + osArch);
                return false;
            }

            StatsigLogger.logInfo("Loading native library: " + resource);
            String libPath = writeLibToTempFile(resource);

            if (libPath == null) {
                return false;
            }

            StatsigLogger.logInfo("Loaded native library: " + libPath);
            System.load(libPath);

            return true;
        } catch (UnsatisfiedLinkError e) {
            StatsigLogger.logError(String.format("Error: Native library not found for the specific OS and architecture. " +
            "Operating System: %s, Architecture: %s. Please ensure that the necessary dependencies have been added to your project configuration.\n",
                    osName, osArch));
            return false;
        }
    }

    private static String writeLibToTempFile(URL resource) {
        try {
            InputStream stream = resource.openStream();

            if (stream == null) {
                StatsigLogger.logError("Unable to open stream for resource: " + resource);
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

            StatsigLogger.logInfo("Successfully created a temporary file for the native library at: " + temp.getAbsolutePath());
            return temp.getAbsolutePath();
        } catch (IOException e) {
            StatsigLogger.logError("I/O Error while writing the library to a temporary file: " + e.getMessage());
            return null;
        }
    }

    private static URL findLibraryResource(String osName, String osArch) {
        ClassLoader cl = StatsigJNI.class.getClassLoader();
        URL resource = null;

        if (osName.contains("win")) {
            if (osArch.equals("x86_64") || osArch.equals("i686") || osArch.equals("aarch64")) {
                resource = cl.getResource("native/libstatsig_ffi.dll");
            }
        } else if (osName.contains("mac")) {
            if (osArch.equals("x86_64") || osArch.equals("amd64") || osArch.equals("aarch64")) {
                resource = cl.getResource("native/libstatsig_ffi.dylib");
            }
        } else if (osName.contains("linux")) {
            if (osArch.equals("x86_64") || osArch.equals("arm64")) {
                resource = cl.getResource("native/libstatsig_ffi.so");
            }
        }

        return resource;
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
