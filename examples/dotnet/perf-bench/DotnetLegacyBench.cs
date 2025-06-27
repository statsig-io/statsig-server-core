using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Statsig;
using Statsig.Server;

namespace perf_bench;

public class DotnetLegacyBench
{
    private const int CORE_ITER = 100_000;
    private const int GCIR_ITER = 1000;
    private static readonly StatsigUser globalUser = new StatsigUser { UserID = "global_user" };
    private static readonly Dictionary<string, double> results = new Dictionary<string, double>();
    private static readonly Random random = new();
    private static readonly string sdkType = "dotnet-server";

    public static async Task Main()
    {
        var sdkKey = Environment.GetEnvironmentVariable("PERF_SDK_KEY");
        if (string.IsNullOrEmpty(sdkKey))
        {
            Console.WriteLine("PERF_SDK_KEY environment variable is not set.");
            return;
        }
        await StatsigServer.Initialize(sdkKey);

        var version = typeof(StatsigServer)
            .Assembly
            .GetName()
            .Version?
            .ToString() ?? "unknown";

        var metadataPath = Environment.GetEnvironmentVariable("BENCH_METADATA_FILE");
        if (!string.IsNullOrEmpty(metadataPath))
        {
            var metadata = $"{{\"sdk_type\": \"{sdkType}\", \"sdk_version\": \"{version}\"}}";
            File.WriteAllText(metadataPath, metadata);
        }

        Console.WriteLine($"Statsig .NET Legacy (v{version})");
        Console.WriteLine("--------------------------------");

        RunBenchmarks(version);

        await StatsigServer.Shutdown();
        Console.WriteLine("\n\n");
    }
    
    private static StatsigUser MakeRandomUser()
    {
        return new StatsigUser { UserID = "user_" + random.Next(1_000_000) };
    }

    private static void Benchmark(string name, Action action, int iterations = CORE_ITER)
    {
        var durations = new List<double>();
        for (int i = 0; i < iterations; i++)
        {
            var sw = Stopwatch.StartNew();
            action();
            sw.Stop();
            durations.Add(sw.Elapsed.TotalMilliseconds);
        }

        durations.Sort();
        int p99Index = (int)(iterations * 0.99);
        double p99 = durations[p99Index];
        results[name] = p99;
    }

    private static void LogBenchmark(string version, string name, double p99)
    {
        Console.WriteLine($"{name,-50} {p99:F4}ms");

        var ci = Environment.GetEnvironmentVariable("CI");
        if (ci != "1" && ci != "true")
        {
            return;
        }

        var metadata = new Dictionary<string, string>
        {
            ["benchmarkName"] = name,
            ["sdkType"] = sdkType,
            ["sdkVersion"] = version
        };
        StatsigServer.LogEvent(globalUser, "sdk_benchmark", p99, metadata);
    }


    private static void RunBenchmarks(string version)
    {
        RunCheckGate();
        RunCheckGateGlobalUser();
        RunGetFeatureGate();
        RunGetFeatureGateGlobalUser();
        RunGetExperiment();
        RunGetExperimentGlobalUser();
        RunGetClientInitializeResponse();
        RunGetClientInitializeResponseGlobalUser();

        foreach (var entry in results)
        {
            LogBenchmark(version, entry.Key, entry.Value);
        }
    }

    private static void RunCheckGate()
    {
        Benchmark("check_gate", () =>
        {
            var user = MakeRandomUser();
            StatsigServer.CheckGateSync(user, "test_advanced");
        });
    }

    private static void RunCheckGateGlobalUser()
    {
        Benchmark("check_gate_global_user", () =>
        {
            StatsigServer.CheckGateSync(globalUser, "test_advanced");
        });
    }

    private static void RunGetFeatureGate()
    {
        Benchmark("get_feature_gate", () =>
        {
            var user = MakeRandomUser();
            StatsigServer.GetFeatureGate(user, "test_advanced");
        });
    }

    private static void RunGetFeatureGateGlobalUser()
    {
        Benchmark("get_feature_gate_global_user", () =>
        {
            StatsigServer.GetFeatureGate(globalUser, "test_advanced");
        });
    }

    private static void RunGetExperiment()
    {
        Benchmark("get_experiment", () =>
        {
            var user = MakeRandomUser();
            StatsigServer.GetExperimentSync(user, "an_experiment");
        });
    }

    private static void RunGetExperimentGlobalUser()
    {
        Benchmark("get_experiment_global_user", () =>
        {
            StatsigServer.GetExperimentSync(globalUser, "an_experiment");
        });
    }

    private static void RunGetClientInitializeResponse()
    {
        Benchmark("get_client_initialize_response", () =>
        {
            var user = MakeRandomUser();
            var res = StatsigServer.GetClientInitializeResponse(user);
            JsonConvert.SerializeObject(res);
        }, GCIR_ITER);
    }

    private static void RunGetClientInitializeResponseGlobalUser()
    {
        Benchmark("get_client_initialize_response_global_user", () =>
        {
            var res = StatsigServer.GetClientInitializeResponse(globalUser);
            JsonConvert.SerializeObject(res);
        }, GCIR_ITER);
    }
}
