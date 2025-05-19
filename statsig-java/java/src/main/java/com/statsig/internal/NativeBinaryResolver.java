package com.statsig.internal;

import java.io.BufferedInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;
import java.util.Properties;

import com.statsig.OutputLogger;
import com.statsig.Statsig;

public class NativeBinaryResolver {
    static final String TAG = "NativeBinaryResolver";
    static final String JAVA_CORE_MAVEN_PATH = "https://repo1.maven.org/maven2/com/statsig/javacore";
    static String SDK_VERSION;

    public static String osName = System.getProperty("os.name").toLowerCase();
    public static String normalizedArch = normalizeArch();

    static {
        try {
            InputStream input = Statsig.class.getClassLoader().getResourceAsStream("statsigsdk.properties");
            Properties props = new Properties();
            props.load(input);
            SDK_VERSION = props.getProperty("version");
        } catch (IOException e) {
            SDK_VERSION = "unknown";
        }
    }

    public static URL findLibraryResource() {
        ClassLoader cl = Statsig.class.getClassLoader();
        URL resource = null;

        String platform = genDetectedPlatform();
        String libName = getLibFileName();

        if (platform != null && libName != null) {
            String resourcePath = "native/" + platform + "/" + libName;
            resource = cl.getResource(resourcePath);
        }

        return resource;
    }


    private static String getLibFileName() {
        if (osName.contains("win")) {
            return "statsig_ffi.dll";
        } else if (osName.contains("mac")) {
            return "libstatsig_ffi.dylib";
        } else if (osName.contains("linux")) {
            return "libstatsig_ffi.so";
        }
        return null;
    }

    private static String genDetectedPlatform() {
        if (osName.contains("win")) {
            return normalizedArch.contains("64") ? "windows-x86_64" : "windows-i686";
        } else if (osName.contains("mac")) {
            return normalizedArch.contains("arm64") ? "macos-arm64" : "macos-x86_64";
        } else if (osName.contains("linux")) {
            String distro = detectLinuxDistro();
            if (distro.equals("amazonlinux2023")) {
                return normalizedArch.contains("arm64") ? "amazonlinux2023-arm64" : "amazonlinux2023-x86_64";
            } else if (distro.equals("amazonlinux2")) {
                return normalizedArch.contains("arm64") ? "amazonlinux2-arm64" : "amazonlinux2-x86_64";
            } else {
                return normalizedArch.contains("arm64") ? "linux-gnu-arm64" : "linux-gnu-x86_64";
            }
        }
        return null;
    }

    private static String normalizeArch() {
        String cpuArch = System.getProperty("os.arch").toLowerCase();
        if (cpuArch.contains("aarch64") || cpuArch.contains("arm64")) {
            return "arm64";
        } else if (cpuArch.contains("x86_64") || cpuArch.contains("amd64")) {
            return "x86_64";
        } else if (cpuArch.contains("i686")) {
            return "i686";
        } else {
            return cpuArch;
        }
    }

    private static String detectLinuxDistro() {
        try {
            List<String> lines = Files.readAllLines(Paths.get("/etc/os-release"));
            String id = null;
            String versionId = null;

            for (String line : lines) {
                if (line.startsWith("ID=")) {
                    id = line.split("=")[1].replace("\"", "").trim();
                } else if (line.startsWith("VERSION_ID=")) {
                    versionId = line.split("=")[1].replace("\"", "").trim();
                }
            }

            OutputLogger.logInfo(TAG, "Parsed /etc/os-release: ID=" + id + ", VERSION_ID=" + versionId);

            if ("amzn".equals(id)) {
                if ("2023".equals(versionId)) {
                    return "amazonlinux2023";
                } else if ("2".equals(versionId)) {
                    return "amazonlinux2";
                }
            }
        } catch (IOException e) {
            OutputLogger.logError(TAG, "Error while detecting linux distro: " + e.getMessage());
        }
        return "linux-gnu";
    }

}
