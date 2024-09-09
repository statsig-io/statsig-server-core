package com.statsig;

class StatsigLogger {
    private static final String LOG_PREFIX = "[Statsig] ";

    static void logError(String message) {
        System.err.println(LOG_PREFIX + message);
    }

    static void logWarning(String message) {
        System.out.println(LOG_PREFIX + "Warning: " + message);
    }

    static void logInfo(String message) {
        System.out.println(LOG_PREFIX + message);
    }

    static void logNativeLibraryError(String osName, String osArch) {
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

        logError(message.toString());
    }
}
