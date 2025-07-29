extern alias StatsigCore;

using System.Reflection;
using System.Diagnostics;
using System.Text.Json;
using StatsigCore::Statsig;
using System.Text.Json.Serialization;

namespace bench;


public class BenchCore
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

    static readonly string sdkVersion = GetVersion();
    static readonly Random rng = new Random();

    const string sdkType = "statsig-server-core-dotnet";
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

    static StatsigCore::Statsig.StatsigUser CreateUser()
    {
        var user = new StatsigUserBuilder()
            .SetUserID("user_" + rng.Next(1_000_000))
            .SetCountry("US")
            .SetLocale("en-US")
            .SetAppVersion("1.0.0")
            .SetUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .AddCustomProperty("isAdmin", false)
            .AddPrivateAttribute("isPaid", "nah")
            .Build();
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
        Console.WriteLine($"Statsig .NET Core (v{sdkVersion})");
        Console.WriteLine("--------------------------------");

        var specNames = LoadSpecNames();

        var options = new StatsigCore::Statsig.StatsigOptionsBuilder().SetSpecsURL("http://scrapi:8000/v2/download_config_specs").SetLogEventURL("http://scrapi:8000/v1/log_event").Build();

        var statsig = new StatsigCore::Statsig.Statsig("secret-DOTNET_CORE", options);
        await statsig.Initialize();

        var results = new List<BenchmarkResult>();
        var globalUser = new StatsigCore::Statsig.StatsigUserBuilder().SetUserID("global_user").Build();

        // Feature gates
        foreach (var gate in specNames["feature_gates"])
        {
            // TODO(weihao): remove the delay
            // Rigth now, we want to make sure check gate works properly
            results.Add(RunBenchmark("check_gate", gate, ITER_HEAVY, () => statsig.CheckGate(CreateUser(), gate)));
            results.Add(RunBenchmark("check_gate_global_user", gate, ITER_HEAVY, () => statsig.CheckGate(globalUser, gate)));
            
            results.Add(RunBenchmark("get_feature_gate", gate, ITER_HEAVY, () => statsig.GetFeatureGate(CreateUser(), gate)));
            await Task.Delay(1);

            results.Add(RunBenchmark("get_feature_gate_global_user", gate, ITER_HEAVY, () => statsig.GetFeatureGate(globalUser, gate)));
            await Task.Delay(1);
        }

        // Dynamic configs
        foreach (var config in specNames["dynamic_configs"])
        {
            results.Add(RunBenchmark("get_dynamic_config", config, ITER_HEAVY, () => statsig.GetDynamicConfig(CreateUser(), config)));
            await Task.Delay(1);
     
            results.Add(RunBenchmark("get_dynamic_config_global_user", config, ITER_HEAVY, () => statsig.GetDynamicConfig(globalUser, config)));
            await Task.Delay(1);
        }

        // Experiments
        foreach (var experiment in specNames["experiments"])
        {
            results.Add(RunBenchmark("get_experiment", experiment, ITER_HEAVY, () => statsig.GetExperiment(CreateUser(), experiment)));
            await Task.Delay(1);
     
            results.Add(RunBenchmark("get_experiment_global_user", experiment, ITER_HEAVY, () => statsig.GetExperiment(globalUser, experiment)));
            await Task.Delay(1);
        }

        // Layers
        foreach (var layer in specNames["layers"])
        {
            results.Add(RunBenchmark("get_layer", layer, ITER_HEAVY, () => statsig.GetLayer(CreateUser(), layer)));
            await Task.Delay(1);
      
            results.Add(RunBenchmark("get_layer_global_user", layer, ITER_HEAVY, () => statsig.GetLayer(globalUser, layer)));
            await Task.Delay(1);
        }

        // Client initialize response (if available in .NET SDK)
        results.Add(RunBenchmark("get_client_initialize_response", "n/a", ITER_LITE, () => statsig.GetClientInitializeResponse(CreateUser())));
        await Task.Delay(1);
    
        results.Add(RunBenchmark("get_client_initialize_response_global_user", "n/a", ITER_LITE, () => statsig.GetClientInitializeResponse(globalUser)));
        await Task.Delay(1);

        await statsig.Shutdown();

        var resultsFile = $"/shared-volume/{sdkType}-{sdkVersion}-results.json";
        File.WriteAllText(resultsFile, JsonSerializer.Serialize(new
        {
            sdkType,
            sdkVersion,
            results
        }, new JsonSerializerOptions { WriteIndented = true }));
    }

    static string GetVersion()
    {
        var custom = typeof(StatsigCore::Statsig.Statsig).Assembly.CustomAttributes;

        foreach (var attribute in custom)
        {
            if (attribute.AttributeType != typeof(AssemblyInformationalVersionAttribute))
            {
                continue;
            }
        
            var value = attribute.ConstructorArguments[0].Value;
            var version = value?.ToString();
            if (version == null)
            {
                throw new Exception("Version is Null");
            }
            var parts = version.Split('+');
            return parts[0];
        } 
        
        throw new Exception("AssemblyInformationalVersionAttribute not Found");
    }
}

