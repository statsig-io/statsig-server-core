import com.statsig.StatsigUser;
import com.statsig.Statsig;

import java.util.*;
import java.util.concurrent.*;

public class JavaCoreBench {
    private static final int ITERATIONS = 100_000;
    private static final StatsigUser globalUser = new StatsigUser.Builder().setUserID("global_user").build();
    private static Map<String, Double> results = new HashMap<>();
    private static Random random = new Random();

    public static void main(String[] args) throws Exception {
        String key = System.getenv("PERF_SDK_KEY");
        Statsig statsig = new Statsig(key);
        statsig.initialize().get();

        System.out.println("Statsig Java Core");
        System.out.println("--------------------------------");

        runCheckGate(statsig);
        runCheckGateGlobalUser(statsig);
        runGetFeatureGate(statsig);
        runGetFeatureGateGlobalUser(statsig);
        runGetExperiment(statsig);
        runGetExperimentGlobalUser(statsig);
        runGetClientInitializeResponse(statsig);
        runGetClientInitializeResponseGlobalUser(statsig);

        for (Map.Entry<String, Double> entry : results.entrySet()) {
            logBenchmark(statsig, entry.getKey(), entry.getValue());
        }

        statsig.shutdown().get();
        System.out.println("\n\n");
    }

    private static StatsigUser makeRandomUser() {
        return new StatsigUser.Builder()
            .setUserID("user_" + random.nextInt(1000000))
            .build();
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

    private static void logBenchmark(Statsig statsig, String name, double p99) {
        System.out.printf("%-50s %.4fms%n", name, p99);
        statsig.logEvent(
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

    private static void runCheckGate(Statsig statsig) {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            if (!statsig.checkGate(user, "test_public")) {
                throw new RuntimeException("Gate check failed");
            }
        });
        results.put("check_gate", p99);
    }

    private static void runCheckGateGlobalUser(Statsig statsig) {
        double p99 = benchmark(() -> {
            statsig.checkGate(globalUser, "test_public");
        });
        results.put("check_gate_global_user", p99);
    }

    private static void runGetFeatureGate(Statsig statsig) {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            statsig.getFeatureGate(user, "test_public");
        });
        results.put("get_feature_gate", p99);
    }

    private static void runGetFeatureGateGlobalUser(Statsig statsig) {
        double p99 = benchmark(() -> {
            statsig.getFeatureGate(globalUser, "test_public");
        });
        results.put("get_feature_gate_global_user", p99);
    }

    private static void runGetExperiment(Statsig statsig) {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            statsig.getExperiment(user, "an_experiment");
        });
        results.put("get_experiment", p99);
    }

    private static void runGetExperimentGlobalUser(Statsig statsig) {
        double p99 = benchmark(() -> {
            statsig.getExperiment(globalUser, "an_experiment");
        });
        results.put("get_experiment_global_user", p99);
    }

    private static void runGetClientInitializeResponse(Statsig statsig) {
        double p99 = benchmark(() -> {
            StatsigUser user = makeRandomUser();
            statsig.getClientInitializeResponse(user);
        });
        results.put("get_client_initialize_response", p99);
    }

    private static void runGetClientInitializeResponseGlobalUser(Statsig statsig) {
        double p99 = benchmark(() -> {
            statsig.getClientInitializeResponse(globalUser);
        });
        results.put("get_client_initialize_response_global_user", p99);
    }
}