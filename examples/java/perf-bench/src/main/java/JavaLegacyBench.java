import com.statsig.FeatureGate;
import com.statsig.sdk.*;
import java.util.*;
import java.util.concurrent.*;

public class JavaLegacyBench {
    private static final int ITERATIONS = 100_000;
    private static final StatsigUser globalUser = new StatsigUser("global_user");
    private static Map<String, Double> results = new HashMap<>();
    private static Random random = new Random();

    public static void main(String[] args) throws Exception {
        String key = System.getenv("PERF_SDK_KEY");
        Statsig.initializeAsync(key).get();

        System.out.println("Statsig Java Legacy");
        System.out.println("--------------------------------");

        runCheckGate();
        runCheckGateGlobalUser();
        runGetFeatureGate();
        runGetFeatureGateGlobalUser();
        runGetExperiment();
        runGetExperimentGlobalUser();
        runGetClientInitializeResponse();
        runGetClientInitializeResponseGlobalUser();

        for (Map.Entry<String, Double> entry : results.entrySet()) {
            logBenchmark( entry.getKey(), entry.getValue());
        }

        Statsig.shutdown();
        System.out.println("\n\n");
    }

    private static StatsigUser makeRandomUser() {
        return new StatsigUser("user_" + random.nextInt(1000000));
    }

    private static double benchmark(Runnable action) {
        List<Double> durations = new ArrayList<>();
        
        for (int i = 0; i < ITERATIONS; i++) {
            long start = System.nanoTime();

            action.run();
            long end = System.nanoTime();
            double durationMs = (end - start) / 1_000_000.0;
            durations.add(durationMs);
        }

        Collections.sort(durations);
        int p99Index = (int) (ITERATIONS * 0.99);
        return durations.get(p99Index);
    }

    private static void logBenchmark(String name, double p99) {
        System.out.printf("%-50s %.4fms%n", name, p99);
        Statsig.logEvent(
            globalUser,
            "sdk_benchmark",
            p99,
            Map.of(
                "benchmarkName", name,
                "sdkType", "statsig-server-core-java",
                "sdkVersion", "1.0.0" // TODO: Get actual version
            )
        );
    }

    private static void runCheckGate() {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            if (!Statsig.checkGateSync(user, "test_public")) {
                throw new RuntimeException("Gate sync failed");
            }
        });
        results.put("check_gate", p99);
    }

    private static void runCheckGateGlobalUser() {
        double p99 = benchmark(() -> {
            Statsig.checkGateSync(globalUser, "test_public");
        });
        results.put("check_gate_global_user", p99);
    }

    private static void runGetFeatureGate() {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            Statsig.getFeatureGate(user, "test_public");
        });
        results.put("get_feature_gate", p99);
    }

    private static void runGetFeatureGateGlobalUser() {
        double p99 = benchmark(() -> {
            Statsig.getFeatureGate(globalUser, "test_public");
        });
        results.put("get_feature_gate_global_user", p99);
    }

    private static void runGetExperiment() {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            Statsig.getExperimentSync(user, "an_experiment");
        });
        results.put("get_experiment", p99);
    }

    private static void runGetExperimentGlobalUser() {
        double p99 = benchmark(() -> {
            Statsig.getExperimentSync(globalUser, "an_experiment");
        });
        results.put("get_experiment_global_user", p99);
    }

    private static void runGetClientInitializeResponse() {
        double p99 = benchmark(() -> {
            Statsig.getClientInitializeResponse(globalUser, HashAlgo.DJB2, null);
        });
        results.put("get_client_initialize_response", p99);
    }

    private static void runGetClientInitializeResponseGlobalUser() {
        double p99 = benchmark(() -> {
            Statsig.getClientInitializeResponse(globalUser, HashAlgo.DJB2, null);
        });
        results.put("get_client_initialize_response_global_user", p99);
    }
}