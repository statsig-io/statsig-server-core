extern alias StatsigLegacy;
extern alias StatsigCore;

using StatsigCore::Statsig;
using StatsigLegacyServer = StatsigLegacy::Statsig.Server.StatsigServer;

public class StatsigWrapper
{
    public static string SCRAPI_URL = "http://scrapi:8000";
    public static bool isCore = false;

    private static StatsigCore.Statsig.Statsig? _statsig;
    private static StatsigCore.Statsig.StatsigUser? _coreUser;
    private static StatsigLegacy.Statsig.StatsigUser? _legacyUser;

    public static async Task Initialize()
    {
        var variant = Environment.GetEnvironmentVariable("SDK_VARIANT");

        if (variant == "core")
        {
            isCore = true;

            var options = new StatsigCore.Statsig.StatsigOptionsBuilder()
                .SetSpecsURL(SCRAPI_URL + "/v2/download_config_specs")
                .SetLogEventURL(SCRAPI_URL + "/v1/log_event")
                .Build();

            _statsig = new StatsigCore.Statsig.Statsig("secret-DOTNET_CORE", options);
            await _statsig.Initialize();
        }
        else if (variant == "legacy")
        {
            isCore = false;

            var options = new StatsigLegacy.Statsig.StatsigOptions
            {
                ApiUrlBase = SCRAPI_URL + "/v1"
            };

            await StatsigLegacyServer.Initialize("secret-DOTNET_LEGACY", options);
        }
        else
        {
            throw new ArgumentException($"Invalid SDK variant: {variant}");
        }
    }

    public static void SetUser(Dictionary<string, string> userData)
    {
        if (isCore)
        {
            _coreUser = new StatsigCore.Statsig.StatsigUserBuilder()
                .SetUserID(userData["userID"])
                .Build();
        }
        else
        {
            _legacyUser = new StatsigLegacy.Statsig.StatsigUser
            {
                UserID = userData["userID"]
            };
        }
    }

    public static bool CheckGate(string gateName)
    {
        if (isCore)
        {
            if (_statsig == null || _coreUser == null) {
                 throw new InvalidOperationException("Statsig not initialized or user not set");
            }
            return _statsig.CheckGate(_coreUser, gateName);
        }
        else
        {
            if (_legacyUser == null) {
                throw new InvalidOperationException("User not set");
            }
            return StatsigLegacyServer.CheckGateSync(_legacyUser, gateName);
        }
    }

    public static void LogEvent(string eventName)
    {
        if (isCore)
        {
            if (_statsig == null || _coreUser == null) {
                throw new InvalidOperationException("Statsig not initialized or user not set");
            }
            _statsig.LogEvent(_coreUser, eventName);
        }
        else
        {
            if (_legacyUser == null) {
                throw new InvalidOperationException("User not set");
            }
            StatsigLegacyServer.LogEvent(_legacyUser, eventName);
        }
    }

    public static string GetClientInitResponse()
    {
        if (isCore)
        {
            if (_statsig == null || _coreUser == null) {
                throw new InvalidOperationException("Statsig not initialized or user not set");
            }
            return _statsig.GetClientInitializeResponse(_coreUser);
        }
        else
        {
            if (_legacyUser == null) {
                throw new InvalidOperationException("User not set");
            }
            return StatsigLegacyServer.GetClientInitializeResponse(_legacyUser).ToString();
        }
    }
}