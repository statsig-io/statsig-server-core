extern alias StatsigLegacy;

using System.Diagnostics;
using System.Text.Json;
using System.Text.Json.Serialization;

using StatsigLegacyServer = StatsigLegacy::Statsig.Server.StatsigServer;

namespace bench;

public class BenchLegacy
{
    public class BenchmarkResult
    {
        [JsonPropertyName("benchmarkName")]
        public string BenchmarkName { get; set; }
        [JsonPropertyName("p99")]
        public double P99 { get; set; }
        [JsonPropertyName("max")]
        public double Max { get; set; }
        [JsonPropertyName("min")]
        public double Min { get; set; }
        [JsonPropertyName("median")]
        public double Median { get; set; }
        [JsonPropertyName("avg")]
        public double Avg { get; set; }
        [JsonPropertyName("specName")]
        public string SpecName { get; set; }
        [JsonPropertyName("sdkType")]
        public string SdkType { get; set; }
        [JsonPropertyName("sdkVersion")]
        public string SdkVersion { get; set; }
    }

    
    static readonly string sdkVersion = typeof(StatsigLegacyServer).Assembly.GetName().Version?.ToString() ?? "unknown";
    static readonly Random rng = new Random();

    const string sdkType = "dotnet-server";
    const string specNamePath = "/shared-volume/spec_names.json";
    const int ITER_LITE = 1000;
    const int ITER_HEAVY = 10_000;

    static Dictionary<string, List<string>> LoadSpecNames()
    {
        for (int i = 0; i < 10; i++)
        {
            if (File.Exists(specNamePath))
                break;
            Task.Delay(1000).Wait();
        }
        var json = File.ReadAllText(specNamePath);
        return JsonSerializer.Deserialize<Dictionary<string, List<string>>>(json);
    }

    static StatsigLegacy::Statsig.StatsigUser CreateUser()
    {
        var user = new StatsigLegacy::Statsig.StatsigUser
        {
            UserID = "user_" + rng.Next(1_000_000),
            Email = "user@example.com",
            IPAddress = "127.0.0.1",
            Locale = "en-US",
            Country = "US",
            AppVersion = "1.0.0",
            UserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        };
        
        user.AddCustomProperty("isAdmin", false);
        user.AddPrivateAttribute("isPaid", "nah");

        return user;
    }

    static BenchmarkResult RunBenchmark(string benchName, string specName, int iterations, Action action)
    {
        var durations = new List<double>(iterations);
        var sw = new Stopwatch();
        for (int i = 0; i < iterations; i++)
        {
            sw.Restart();
            action();
            sw.Stop();
            durations.Add(sw.Elapsed.TotalMilliseconds);
        }
        
        durations.Sort();
        int p99Index = (int)(iterations * 0.99);
        
        var result = new BenchmarkResult
        {
            BenchmarkName = benchName,
            P99 = durations[Math.Min(p99Index, durations.Count - 1)],
            Max = durations.Last(),
            Min = durations.First(),
            Median = durations[durations.Count / 2],
            Avg = durations.Average(),
            SpecName = specName,
            SdkType = sdkType,
            SdkVersion = sdkVersion
        };


        Console.WriteLine($"{result.BenchmarkName.PadRight(30)} p99({result.P99.ToString("F4")}ms) max({result.Max.ToString("F4")}ms) {result.SpecName}");

        return result;
    }

    public static async Task Run()
    {
        Console.WriteLine($"Statsig .NET Legacy (v{sdkVersion})");
        Console.WriteLine("--------------------------------");

        var specNames = LoadSpecNames();

        var options = new StatsigLegacy::Statsig.StatsigServerOptions(apiUrlBase: "http://scrapi:8000/v1");

        await StatsigLegacyServer.Initialize("secret-DOTNET_LEGACY", options);

        var results = new List<BenchmarkResult>();
        var globalUser = new StatsigLegacy::Statsig.StatsigUser { UserID = "global_user" };

        // Feature gates
        foreach (var gate in specNames["feature_gates"])
        {
            results.Add(RunBenchmark("check_gate", gate, ITER_HEAVY, () => StatsigLegacyServer.CheckGateSync(CreateUser(), gate)));
            results.Add(RunBenchmark("check_gate_global_user", gate, ITER_HEAVY, () => StatsigLegacyServer.CheckGateSync(globalUser, gate)));
            
            results.Add(RunBenchmark("get_feature_gate", gate, ITER_HEAVY, () => StatsigLegacyServer.GetFeatureGate(CreateUser(), gate)));
            results.Add(RunBenchmark("get_feature_gate_global_user", gate, ITER_HEAVY, () => StatsigLegacyServer.GetFeatureGate(globalUser, gate)));
        }

        // Dynamic configs
        foreach (var config in specNames["dynamic_configs"])
        {
            results.Add(RunBenchmark("get_dynamic_config", config, ITER_HEAVY, () => StatsigLegacyServer.GetConfigSync(CreateUser(), config)));
            results.Add(RunBenchmark("get_dynamic_config_global_user", config, ITER_HEAVY, () => StatsigLegacyServer.GetConfigSync(globalUser, config)));
        }

        // Experiments
        foreach (var experiment in specNames["experiments"])
        {
            results.Add(RunBenchmark("get_experiment", experiment, ITER_HEAVY, () => StatsigLegacyServer.GetExperimentSync(CreateUser(), experiment)));
            results.Add(RunBenchmark("get_experiment_global_user", experiment, ITER_HEAVY, () => StatsigLegacyServer.GetExperimentSync(globalUser, experiment)));
        }

        // Layers
        foreach (var layer in specNames["layers"])
        {
            results.Add(RunBenchmark("get_layer", layer, ITER_HEAVY, () => StatsigLegacyServer.GetLayerSync(CreateUser(), layer)));
            results.Add(RunBenchmark("get_layer_global_user", layer, ITER_HEAVY, () => StatsigLegacyServer.GetLayerSync(globalUser, layer)));
        }

        // Client initialize response (if available in .NET SDK)
        results.Add(RunBenchmark("get_client_initialize_response", "n/a", ITER_LITE, () => StatsigLegacyServer.GetClientInitializeResponse(CreateUser())));
        results.Add(RunBenchmark("get_client_initialize_response_global_user", "n/a", ITER_LITE, () => StatsigLegacyServer.GetClientInitializeResponse(globalUser)));

        await StatsigLegacyServer.Shutdown();

        var resultsFile = $"/shared-volume/{sdkType}-{sdkVersion}-results.json";
        File.WriteAllText(resultsFile, JsonSerializer.Serialize(new
        {
            sdkType,
            sdkVersion,
            results
        }, new JsonSerializerOptions { WriteIndented = true }));
    }
}