package com.statsig;

import java.io.*;
import java.net.URL;
import java.util.Map;

class StatsigJNI {
    private static final boolean libraryLoaded;
    private static final String logContext = "com.statsig.StatsigJNI";

    static boolean isLibraryLoaded() {
        return libraryLoaded;
    }

    static {
        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();

        OutputLogger.logInfo(logContext, "Detected OS: " + osName + " Arch: " + osArch);

        libraryLoaded = loadNativeLibrary(osName, osArch);

        if (!libraryLoaded) {
            logNativeLibraryError(osName, osArch);
        }
    }

    private static boolean loadNativeLibrary(String osName, String osArch) {
        try {
            URL resource = findLibraryResource(osName, osArch);

            if (resource == null) {
                OutputLogger.logError(
                        logContext,
                        "Unable to find native library resource for OS: " + osName + " Arch: " + osArch
                );
                return false;
            }

            OutputLogger.logInfo(
                    logContext,
                    "Loading native library: " + resource
            );
            String libPath = writeLibToTempFile(resource);

            if (libPath == null) {
                return false;
            }

            OutputLogger.logInfo(
                    logContext,
                    "Loaded native library: " + libPath
            );
            System.load(libPath);

            return true;
        } catch (UnsatisfiedLinkError e) {
            OutputLogger.logError(
                    logContext,
                    String.format("Error: Native library not found for the specific OS and architecture. " +
                                    "Operating System: %s, Architecture: %s. Please ensure that the necessary dependencies have been added to your project configuration.\n",
                            osName, osArch));
            return false;
        }
    }

    private static String writeLibToTempFile(URL resource) {
        try {
            InputStream stream = resource.openStream();

            if (stream == null) {
                OutputLogger.logError(logContext,"Unable to open stream for resource: " + resource);
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

            OutputLogger.logInfo(logContext,"Successfully created a temporary file for the native library at: " + temp.getAbsolutePath());
            return temp.getAbsolutePath();
        } catch (IOException e) {
            OutputLogger.logError(logContext,"I/O Error while writing the library to a temporary file: " + e.getMessage());
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

    private static void logNativeLibraryError(String osName, String osArch) {
        StringBuilder message = new StringBuilder();

        message.append("To resolve this issue, ensure that the correct native library is available for your platform.\n");

        String normalizedOsName = osName.toLowerCase().replace(" ", "");

        if (normalizedOsName.contains("macos")) {
            if (osArch.contains("aarch64")) {
                message.append("For macOS with ARM64 architecture, add the following dependency to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:macos-aarch64'\n");
            } else if (osArch.contains("x86_64")) {
                message.append("For macOS with x86_64 architecture, add the following dependency to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:macos-x86_64'\n");
            }
        } else if (normalizedOsName.contains("linux")) {
            if (osArch.contains("arm64")) {
                message.append("For Linux with ARM64 architecture, add one of the following dependencies to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:amazonlinux2-arm64'\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:amazonlinux2023-arm64'\n");
            } else if (osArch.contains("x86_64")) {
                message.append("For Linux with x86_64 architecture, add one of the following dependencies to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:amazonlinux2-x86_64'\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:amazonlinux2023-x86_64'\n");
            }
        } else if (normalizedOsName.contains("win")) {
            if (osArch.contains("aarch64")) {
                message.append("For Windows with ARM64 architecture, add the following dependency to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:windows-aarch64'\n");
            } else if (osArch.contains("i686")) {
                message.append("For Windows with i686 architecture, add the following dependency to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:windows-i686'\n");
            } else if (osArch.contains("x86_64")) {
                message.append("For Windows with x86_64 architecture, add the following dependency to your build.gradle file:\n");
                message.append("  implementation 'com.statsig:serversdk-test:<version>:windows-x86_64'\n");
            }
        } else {
            message.append(String.format("Warning: Unsupported or unknown operating system '%s'. Please check your environment.\n", osName));
        }
        message.append("If you continue to experience issues, refer to the official documentation or contact support.\n");

        OutputLogger.logError(logContext, message.toString());
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
            String environment,
            long outputLoggerLevel
    );

    public static native void statsigOptionsRelease(int optionsRef);
}
