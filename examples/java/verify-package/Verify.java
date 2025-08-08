import java.net.URI;
import java.net.http.*;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;
import com.statsig.*;

public class Verify {

    public static void main(String[] args) throws Exception {
        String sdkKey = System.getenv("STATSIG_SERVER_SDK_KEY");
        if (sdkKey == null || sdkKey.isEmpty()) {
            throw new Error("STATSIG_SERVER_SDK_KEY is not set");
        }

        Statsig statsig = new Statsig(sdkKey);
        statsig.initialize().get();

        StatsigUser user = new StatsigUser.Builder()
                .setUserID("a_user")
                .setCustom(Map.of(
                        "os", System.getProperty("os.name"),
                        "arch", System.getProperty("os.arch"),
                        "nodeVersion", System.getProperty("java.version")
                ))
                .build();

        boolean gate = statsig.checkGate(user, "test_public");
        String gcir = statsig.getClientInitializeResponse(user);

        System.out.println(
                "-------------------------------- Get Client Initialize Response --------------------------------"
        );
        System.out.println(JSON.toJSONString(JSON.parseObject(gcir), JSONWriter.Feature.PrettyFormat));
        System.out.println(
                "-------------------------------------------------------------------------------------------------"
        );

        System.out.println("Gate test_public: " + gate);

        if (!gate) {
            throw new Error("\"test_public\" gate is false but should be true");
        }

        Map<String, Object> gcirJson = JSON.parseObject(gcir, Map.class);
        if (gcirJson.size() < 1) {
            throw new Error("GCIR is missing required fields");
        }

        System.out.println("All checks passed, shutting down...");
        statsig.shutdown();
        System.out.println("Shutdown complete");
    }
}
