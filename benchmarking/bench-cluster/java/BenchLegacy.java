import java.util.*;
import java.util.concurrent.*;
import java.io.*;

public class BenchLegacy {
    public static void main(String[] args) throws Exception {
        String sdkVersion = getSdkVersion();
        System.out.println("Hello, World! " + sdkVersion);
    }

    private static String getSdkVersion() throws Exception {
        String root = System.getProperty("user.dir");
        Properties props = new Properties();
        props.load(new FileInputStream(root + "/build/versions.properties"));
        String version = props.getProperty("legacy.version", "unknown");
        return version;
    }
}