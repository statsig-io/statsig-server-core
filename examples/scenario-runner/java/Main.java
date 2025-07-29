import java.net.URI;
import java.net.http.*;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;
import com.alibaba.fastjson2.JSON;

public class Main {

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

    static void update() throws Exception {
        System.out.println("--------------------------------------- [ Update ]");
        SdkState state = readState();

        int logEventQps = state.logEvent.qps;
        int gateQps = state.gate.qps;

        System.out.println("Users: " + state.users.size());
        System.out.println("Gates: " + state.gate.names.size() + " qps: " + gateQps);
        System.out.println("Events: " + state.logEvent.events.size() + " qps: " + logEventQps);

        for (Map<String, String> user : state.users) {
            StatsigWrapper.setUser(user);

            for (String gateName : state.gate.names) {
                for (int i = 0; i < gateQps; i++) {
                    StatsigWrapper.checkGate(gateName);
                }
            }

            for (Map<String, String> event : state.logEvent.events) {   
                for (int i = 0; i < logEventQps; i++) {
                    StatsigWrapper.logEvent(event.get("eventName"));
                }
            }
        }
    }

    static SdkState readState() throws Exception {
        String contents = Files.readString(Paths.get("/shared-volume/state.json"));
//        String contents = Files.readString(Paths.get("/Users/danielloomb/Projects/kong/bridges/core-napi-bridge/sdk/examples/scenario-runner/shared-volume/state.json"));

        return JSON.parseObject(contents, State.class).sdk;
    }
}

class State {
    public SdkState sdk;
}

class SdkState {
    public List<Map<String, String>> users;
    public GateConfig gate;
    public LogEventConfig logEvent;
}

class GateConfig {
    public List<String> names;
    public int qps;
}

class LogEventConfig {
    public List<Map<String, String>> events;
    public int qps;
}

