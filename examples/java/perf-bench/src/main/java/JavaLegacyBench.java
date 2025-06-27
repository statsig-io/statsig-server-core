import com.alibaba.fastjson2.JSON;
import com.statsig.FeatureGate;
import com.statsig.sdk.*;
import java.util.*;
import java.util.concurrent.*;
import java.io.*;

public class JavaLegacyBench {
    private static final int CORE_ITER = 100_000;
    private static final int GCIR_ITER = 1000;
    private static final StatsigUser globalUser = new StatsigUser("global_user");
    private static Map<String, Double> results = new HashMap<>();
    private static Random random = new Random();
    private static String sdk_type = "java-server";

    public static void main(String[] args) throws Exception {
        String key = System.getenv("PERF_SDK_KEY");
        Statsig.initializeAsync(key).get();

        Properties props = new Properties();
        props.load(new FileInputStream("build/versions.properties"));
        String version = props.getProperty("legacy.version", "unknown");

        String metadataFile = System.getenv("BENCH_METADATA_FILE");
        try (FileWriter writer = new FileWriter(metadataFile)) {
            writer.write(String.format("{\"sdk_type\": \"%s\", \"sdk_version\": \"%s\"}", sdk_type, version));
        }

        System.out.println("Statsig Java Legacy (v" + version + ")");
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
            logBenchmark(version, entry.getKey(), entry.getValue());
        }

        Statsig.shutdown();
        System.out.println("\n\n");
    }

    private static StatsigUser makeRandomUser() {
        return new StatsigUser("user_" + random.nextInt(1000000));
    }

    private static double benchmark(int iterations, Runnable action) {
        List<Double> durations = new ArrayList<>();
        
        for (int i = 0; i < iterations; i++) {
            long start = System.nanoTime();

            action.run();
            long end = System.nanoTime();
            double durationMs = (end - start) / 1_000_000.0;
            durations.add(durationMs);
        }

        Collections.sort(durations);
        int p99Index = (int) (iterations * 0.99);
        return durations.get(p99Index);
    }

    private static void logBenchmark(String version, String name, double p99) {
        System.out.printf("%-50s %.4fms%n", name, p99);

        String ci = System.getenv("CI");
        if (!Objects.equals(ci, "1") && !Objects.equals(ci, "true")) {
            return;
        }

        Statsig.logEvent(
            globalUser,
            "sdk_benchmark",
            p99,
            Map.of(
                "benchmarkName", name,
                "sdkType", "java-server",
                "sdkVersion", version
            )
        );
    }

    private static void runCheckGate() {
        double p99 = benchmark(CORE_ITER, () -> {
            StatsigUser user = makeRandomUser();
            Statsig.checkGateSync(user, "test_advanced");
        });
        results.put("check_gate", p99);
    }

    private static void runCheckGateGlobalUser() {
        double p99 = benchmark(CORE_ITER, () -> {
            Statsig.checkGateSync(globalUser, "test_advanced");
        });
        results.put("check_gate_global_user", p99);
    }

    private static void runGetFeatureGate() {
        double p99 = benchmark(CORE_ITER, () -> {
            StatsigUser user = makeRandomUser();
            Statsig.getFeatureGate(user, "test_advanced");
        });
        results.put("get_feature_gate", p99);
    }

    private static void runGetFeatureGateGlobalUser() {
        double p99 = benchmark(CORE_ITER, () -> {
            Statsig.getFeatureGate(globalUser, "test_advanced");
        });
        results.put("get_feature_gate_global_user", p99);
    }

    private static void runGetExperiment() {
        double p99 = benchmark(CORE_ITER, () -> {
            StatsigUser user = makeRandomUser();
            Statsig.getExperimentSync(user, "an_experiment");
        });
        results.put("get_experiment", p99);
    }

    private static void runGetExperimentGlobalUser() {
        double p99 = benchmark(CORE_ITER, () -> {
            Statsig.getExperimentSync(globalUser, "an_experiment");
        });
        results.put("get_experiment_global_user", p99);
    }

    private static void runGetClientInitializeResponse() {
        double p99 = benchmark(GCIR_ITER, () -> {
            Map<String, Object> res = Statsig.getClientInitializeResponse(globalUser, HashAlgo.DJB2, null);
            JSON.toJSONString(res);
        });
        results.put("get_client_initialize_response", p99);
    }

    private static void runGetClientInitializeResponseGlobalUser() {
        double p99 = benchmark(GCIR_ITER, () -> {
            Map<String, Object> res = Statsig.getClientInitializeResponse(globalUser, HashAlgo.DJB2, null);
            JSON.toJSONString(res);
        });
        results.put("get_client_initialize_response_global_user", p99);
    }
}