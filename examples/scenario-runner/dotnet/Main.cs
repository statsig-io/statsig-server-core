using System.Diagnostics;
using System.Text.Json;

public class Program
{
    private static readonly List<ProfileResult> ProfileArr = new();

    public static async Task Main(string[] args)
    {
        await WaitForScrapi();
        await StatsigWrapper.Initialize();

        while (true)
        {
            await Update();
            await Task.Delay(1000);
        }
    }

    private static async Task WaitForScrapi()
    {
        using var client = new HttpClient();
        for (int i = 0; i < 10; i++)
        {
            try
            {
                var response = await client.GetAsync($"{StatsigWrapper.SCRAPI_URL}/ready");
                if (response.IsSuccessStatusCode)
                {
                    break;
                }
            }
            catch
            {
                // noop
            }

            Console.WriteLine("Waiting for scrapi to be ready");
            await Task.Delay(1000);
        }
    }

    private static void Profile(string name, string userID, string extra, int QPS, Action fn)
    {
        var durations = new double[QPS];
        for (int i = 0; i < QPS; i++)
        {
            var stopwatch = Stopwatch.StartNew();
            fn();
            stopwatch.Stop();
            durations[i] = stopwatch.Elapsed.TotalMilliseconds;
        }

        Array.Sort(durations);

        var result = new ProfileResult
        {
            Name = name,
            UserID = userID,
            Extra = extra,
            QPS = QPS
        };

        if (QPS > 0)
        {
            var median = durations[QPS / 2];
            var p99 = durations[QPS * 99 / 100];
            var min = durations[0];
            var max = durations[QPS - 1];

            Console.WriteLine($"{name} took {p99:F4}ms (p99), {max:F4}ms (max)");

            result.Median = median;
            result.P99 = p99;
            result.Min = min;
            result.Max = max;
        }

        ProfileArr.Add(result);
    }

    private static async Task Update()
    {
        Console.WriteLine("--------------------------------------- [ Update ]");

        var state = ReadState();
        ProfileArr.Clear();

        var logEventQPS = state.Sdk.LogEvent.QPS;
        var gateQPS = state.Sdk.Gate.QPS;

        Console.WriteLine($"Users: {state.Sdk.Users.Count}");
        Console.WriteLine($"Gates: {state.Sdk.Gate.Names.Count} QPS: {gateQPS}");
        Console.WriteLine($"Events: {state.Sdk.LogEvent.Events.Count} QPS: {logEventQPS}");

        foreach (var user in state.Sdk.Users.Values)
        {
            StatsigWrapper.SetUser(user);

            foreach (var gateName in state.Sdk.Gate.Names)
            {
                Profile("check_gate", user["userID"], gateName, gateQPS, 
                    () => StatsigWrapper.CheckGate(gateName));
            }

            foreach (var eventData in state.Sdk.LogEvent.Events.Values)
            {
                Profile("log_event", user["userID"], eventData["eventName"], logEventQPS, 
                    () => StatsigWrapper.LogEvent(eventData["eventName"]));
            }

            Profile("gcir", user["userID"], "", state.Sdk.Gcir.QPS, 
                () => StatsigWrapper.GetClientInitResponse());
        }

        await WriteProfileData();
    }

    private static State ReadState()
    {
        var contents = File.ReadAllText("/shared-volume/state.json");
        return JsonSerializer.Deserialize<State>(contents) ?? throw new InvalidOperationException("Failed to deserialize state");
    }

    private static async Task WriteProfileData()
    {
        var data = JsonSerializer.Serialize(ProfileArr, new JsonSerializerOptions { WriteIndented = true });
        var slug = $"profile-dotnet-{(StatsigWrapper.isCore ? "core" : "legacy")}";
        var tempPath = $"/shared-volume/{slug}-temp.json";
        var finalPath = $"/shared-volume/{slug}.json";

        await File.WriteAllTextAsync(tempPath, data);
        File.Move(tempPath, finalPath, true);
    }
}

public class State
{
    public SdkState Sdk { get; set; } = new();
}

public class SdkState
{
    public Dictionary<string, Dictionary<string, string>> Users { get; set; } = new();
    public GateConfig Gate { get; set; } = new();
    public LogEventConfig LogEvent { get; set; } = new();
    public GcirConfig Gcir { get; set; } = new();
}

public class GateConfig
{
    public List<string> Names { get; set; } = new();
    public int QPS { get; set; }
}

public class LogEventConfig
{
    public Dictionary<string, Dictionary<string, string>> Events { get; set; } = new();
    public int QPS { get; set; }
}

public class GcirConfig
{
    public int QPS { get; set; }
}

public class ProfileResult
{
    public string Name { get; set; } = "";
    public string UserID { get; set; } = "";
    public string Extra { get; set; } = "";
    public int QPS { get; set; }
    public double? Median { get; set; }
    public double? P99 { get; set; }
    public double? Min { get; set; }
    public double? Max { get; set; }
}
