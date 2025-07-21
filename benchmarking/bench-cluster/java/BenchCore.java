import java.util.*;
import java.util.concurrent.*;
import java.io.*;
import java.nio.file.*;
import com.statsig.*;
import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;

public class BenchCore {
    static final String sdkType = "statsig-server-core-java";
    static final String SCRAPI_URL = "http://scrapi:8000";
    static final int ITER_LITE = 1000;
    static final int ITER_HEAVY = 10_000;
    static final Random random = new Random();

    public static void main(String[] args) throws Exception {
        String sdkVersion = getSdkVersion();
        System.out.println("Statsig Java Core (v" + sdkVersion + ")");
        System.out.println("--------------------------------");

        Map<String, List<String>> specNames = loadSpecNames();

        StatsigOptions options = new StatsigOptions.Builder()
            .setSpecsUrl(SCRAPI_URL + "/v2/download_config_specs")
            .setLogEventUrl(SCRAPI_URL + "/v1/log_event")
            .build();

        Statsig statsig = new Statsig("secret-JAVA_CORE", options);
        statsig.initialize().get();

        List<BenchmarkResult> results = new ArrayList<>();

        // Create a global user
        StatsigUser globalUser = new StatsigUser.Builder().setUserID("global_user").build();

        // Feature gates
        for (String gate : specNames.get("feature_gates")) {
            benchmark("check_gate", gate, ITER_HEAVY, () -> statsig.checkGate(createUser(), gate), results, sdkVersion);
            benchmark("check_gate_global_user", gate, ITER_HEAVY, () -> statsig.checkGate(globalUser, gate), results, sdkVersion);
            benchmark("get_feature_gate", gate, ITER_HEAVY, () -> statsig.getFeatureGate(createUser(), gate), results, sdkVersion);
            benchmark("get_feature_gate_global_user", gate, ITER_HEAVY, () -> statsig.getFeatureGate(globalUser, gate), results, sdkVersion);
        }

        // Dynamic configs
        for (String config : specNames.get("dynamic_configs")) {
            benchmark("get_dynamic_config", config, ITER_HEAVY, () -> statsig.getDynamicConfig(createUser(), config), results, sdkVersion);
            benchmark("get_dynamic_config_global_user", config, ITER_HEAVY, () -> statsig.getDynamicConfig(globalUser, config), results, sdkVersion);
        }

        // Experiments
        for (String experiment : specNames.get("experiments")) {
            benchmark("get_experiment", experiment, ITER_HEAVY, () -> statsig.getExperiment(createUser(), experiment), results, sdkVersion);
            benchmark("get_experiment_global_user", experiment, ITER_HEAVY, () -> statsig.getExperiment(globalUser, experiment), results, sdkVersion);
        }

        // Layers
        for (String layer : specNames.get("layers")) {
            benchmark("get_layer", layer, ITER_HEAVY, () -> statsig.getLayer(createUser(), layer), results, sdkVersion);
            benchmark("get_layer_global_user", layer, ITER_HEAVY, () -> statsig.getLayer(globalUser, layer), results, sdkVersion);
        }

        // Client initialize response
        benchmark("get_client_initialize_response", "n/a", ITER_LITE, () -> statsig.getClientInitializeResponse(createUser()), results, sdkVersion);
        benchmark("get_client_initialize_response_global_user", "n/a", ITER_LITE, () -> statsig.getClientInitializeResponse(globalUser), results, sdkVersion);

        statsig.shutdown().get();

        // Write results
        writeResults(sdkType, sdkVersion, results);
    }

    private static Map<String, List<String>> loadSpecNames() throws Exception {
        String path = "/shared-volume/spec_names.json";
        for (int i = 0; i < 10; i++) {
            if (Files.exists(Paths.get(path))) break;
            Thread.sleep(1000);
        }
        String json = Files.readString(Paths.get(path));
        return JSON.parseObject(json, Map.class);
    }

    private static StatsigUser createUser() {
        return new StatsigUser.Builder()
            .setUserID("user_" + random.nextInt(1000000))
            .build();
    }


    // --- Benchmarking logic ---
    private static void benchmark(String benchName, String specName, int iterations, Runnable func, List<BenchmarkResult> results, String sdkVersion) {
        List<Double> durations = new ArrayList<>();
        for (int i = 0; i < iterations; i++) {
            long start = System.nanoTime();
            func.run();
            long end = System.nanoTime();
            durations.add((end - start) / 1_000_000.0); // ms
        }
        Collections.sort(durations);
        int p99Index = (int)(iterations * 0.99);
        BenchmarkResult result = new BenchmarkResult(
            benchName,
            durations.get(p99Index),
            durations.get(durations.size() - 1),
            durations.get(0),
            durations.get(durations.size() / 2),
            durations.stream().mapToDouble(Double::doubleValue).average().orElse(0.0),
            specName,
            sdkType,
            sdkVersion
        );
        results.add(result);
        System.out.printf("%-30s p99(%.4fms) max(%.4fms) %s\n", benchName, result.p99, result.max, specName);
        try { Thread.sleep(1); } catch (InterruptedException ignored) {}
    }

    private static void writeResults(String sdkType, String sdkVersion, List<BenchmarkResult> results) throws Exception {
        HashMap<String, Object> root = new HashMap<>();
        root.put("sdkType", sdkType);
        root.put("sdkVersion", sdkVersion);
        root.put("results", results);
        String outPath = String.format("/shared-volume/%s-%s-results.json", sdkType, sdkVersion);
        Files.writeString(Paths.get(outPath), JSON.toJSONString(root, JSONWriter.Feature.PrettyFormat));
    }

    private static String getSdkVersion() throws Exception {
        String root = System.getProperty("user.dir");
        Properties props = new Properties();
        props.load(new FileInputStream(root + "/build/versions.properties"));
        String version = props.getProperty("core.version", "unknown");
        return version;
    }

    // --- BenchmarkResult class ---
    static class BenchmarkResult {
        public String benchmarkName;
        public double p99;
        public double max;
        public double min;
        public double median;
        public double avg;
        public String specName;
        public String sdkType;
        public String sdkVersion;

        public BenchmarkResult(String benchmarkName, double p99, double max, double min, double median, double avg, String specName, String sdkType, String sdkVersion) {
            this.benchmarkName = benchmarkName;
            this.p99 = p99;
            this.max = max;
            this.min = min;
            this.median = median;
            this.avg = avg;
            this.specName = specName;
            this.sdkType = sdkType;
            this.sdkVersion = sdkVersion;
        }
    }
}