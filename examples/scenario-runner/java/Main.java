import java.net.URI;
import java.net.http.*;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;
import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;

public class Main {

    static List<Map<String, Object>> PROFILE_ARR = new ArrayList<>();

    public static void main(String[] args) throws Exception {
        waitForScrapi();

        StatsigWrapper.initialize();

        while (true) {
            update();
            Thread.sleep(1000);
        }
    }

    static void waitForScrapi() throws Exception {
        HttpClient client = HttpClient.newHttpClient();
        for (int i = 0; i < 10; i++) {
            try {
                HttpRequest request = HttpRequest.newBuilder()
                        .uri(URI.create(StatsigWrapper.SCRAPI_URL + "/ready"))
                        .timeout(Duration.ofSeconds(1))
                        .build();
                HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
                if (response.statusCode() == 200) {
                    break;
                }
            } catch (Exception e) {
                // noop
            }

            System.out.println("Waiting for scrapi to be ready");
            Thread.sleep(1000);
        }
    }

    static void profile(String name, String userID, String extra, int qps, Runnable fn) {
        double[] durations = new double[qps];
        for (int i = 0; i < qps; i++) {
            long start = System.nanoTime();
            fn.run();
            long end = System.nanoTime();
            durations[i] = (end - start) / 1_000_000.0; // ms
        }

        Arrays.sort(durations);

        Map<String, Object> results = new HashMap<>();
        results.put("name", name);
        results.put("userID", userID);
        results.put("extra", extra);
        results.put("qps", qps);

        if (qps > 0) {
            double median = durations[qps / 2];
            double p99 = durations[qps * 99 / 100];
            double min = durations[0];
            double max = durations[qps - 1];

            System.out.println(name + " took " +  p99 + "ms (p99), " +  max + "ms (max)");

            results.put("median", median);
            results.put("p99", p99);
            results.put("min", min);
            results.put("max", max);
        }

        PROFILE_ARR.add(results);
    }

    static void update() throws Exception {
        System.out.println("--------------------------------------- [ Update ]");
        SdkState state = readState();

        PROFILE_ARR.clear();

        int logEventQps = state.logEvent.qps;
        int gateQps = state.gate.qps;

        System.out.println("Users: " + state.users.size());
        System.out.println("Gates: " + state.gate.names.size() + " qps: " + gateQps);
        System.out.println("Events: " + state.logEvent.events.size() + " qps: " + logEventQps);

        for (Map<String, String> user : state.users.values()) {
            StatsigWrapper.setUser(user);
    
            for (String gateName : state.gate.names) {
                profile("check_gate", user.get("userID"), gateName, gateQps, () -> StatsigWrapper.checkGate(gateName));
            }

            for (Map<String, String> event : state.logEvent.events.values()) {
                profile("log_event", user.get("userID"), event.get("eventName"), logEventQps, () -> StatsigWrapper.logEvent(event.get("eventName")));
            }

            profile("gcir", user.get("userID"), "", state.gcir.qps, () -> StatsigWrapper.getClientInitResponse());
        }

        writeProfileData();
    }

    static SdkState readState() throws Exception {
        String contents = Files.readString(Paths.get("/shared-volume/state.json"));
//        String contents = Files.readString(Paths.get("/Users/danielloomb/Projects/kong/bridges/core-napi-bridge/sdk/examples/scenario-runner/shared-volume/state.json"));

        return JSON.parseObject(contents, State.class).sdk;
    }

    static void writeProfileData() throws Exception {
        String prettyJson = JSON.toJSONString(PROFILE_ARR, JSONWriter.Feature.PrettyFormat);
        String slug = "profile-java-" + (StatsigWrapper.isCore ? "core" : "legacy");
        Files.writeString(Paths.get("/shared-volume/" + slug + "-temp.json"), prettyJson);
        Runtime.getRuntime().exec("mv /shared-volume/" + slug + "-temp.json /shared-volume/" + slug + ".json");
    }
}

class State {
    public SdkState sdk;
}

class SdkState {
    public Map<String, Map<String, String>> users;
    public GateConfig gate;
    public LogEventConfig logEvent;
    public GcirConfig gcir;
}

class GateConfig {
    public List<String> names;
    public int qps;
}

class LogEventConfig {
    public Map<String, Map<String, String>> events;
    public int qps;
}

class GcirConfig {
    public int qps;
}
