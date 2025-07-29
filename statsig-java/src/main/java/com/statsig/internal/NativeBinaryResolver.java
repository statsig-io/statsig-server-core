package com.statsig.internal;

import com.statsig.OutputLogger;
import com.statsig.Statsig;
import java.io.*;
import java.net.URL;
import java.util.Properties;

public class NativeBinaryResolver {
  static final String TAG = "NativeBinaryResolver";
  static String sdkVersion;

  public static String osName = System.getProperty("os.name").toLowerCase();
  public static String normalizedArch = normalizeArch();

  static {
    try {
      InputStream input =
          Statsig.class.getClassLoader().getResourceAsStream("statsigsdk.properties");
      Properties props = new Properties();
      props.load(input);
      sdkVersion = props.getProperty("version");
    } catch (IOException e) {
      sdkVersion = "unknown";
    }
  }

  /** [Internal] Library Loading */
  public static boolean load() {
    boolean loaded = loadNativeLibraryFromResources();

    if (!loaded) {
      logNativeLibraryError();
    }

    return loaded;
  }

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

  private static URL findLibraryResource() {
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
      String dependencyName = detectLibc().equals("musl") ? "linux-musl" : "linux-gnu";
      addDependency(message, "Linux", arch, dependencyName);
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
      return normalizedArch.contains("64") ? "x86_64-pc-windows-msvc" : "i686-pc-windows-msvc";
    } else if (osName.contains("mac")) {
      return normalizedArch.contains("arm64") ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
    } else if (osName.contains("linux")) {
      String libcName = detectLibc();
      if (libcName.equals("musl")) {
        return normalizedArch.contains("arm64")
            ? "aarch64-unknown-linux-musl"
            : "x86_64-unknown-linux-musl";
      } else if (libcName.equals("gnu")) {
        return normalizedArch.contains("arm64")
            ? "aarch64-unknown-linux-gnu"
            : "x86_64-unknown-linux-gnu";
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

  private static String detectLibc() {
    try {
      Process process = Runtime.getRuntime().exec("ldd --version");
      BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()));

      String line;
      while ((line = reader.readLine()) != null) {
        if (line.toLowerCase().contains("musl")) {
          return "musl";
        } else if (line.toLowerCase().contains("gnu libc")
            || line.toLowerCase().contains("glibc")) {
          return "gnu";
        }
      }
    } catch (IOException e) {
      OutputLogger.logError(TAG, "Error while detecting linux distro: " + e.getMessage());
    }
    return "gnu";
  }
}
