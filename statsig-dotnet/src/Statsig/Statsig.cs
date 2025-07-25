using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using System.Collections.Generic;
using System.Reflection;

namespace Statsig
{
    public class Statsig : IDisposable
    {
        private readonly unsafe ulong _statsigRef;

        public Statsig(string sdkKey, StatsigOptions options)
        {
            var sdkKeyBytes = Encoding.UTF8.GetBytes(sdkKey);
            UpdateStatsigMetadata();
            unsafe
            {
                fixed (byte* sdkKeyPtr = sdkKeyBytes)
                {
                    _statsigRef = StatsigFFI.statsig_create(sdkKeyPtr, options.Reference);
                }
            }
        }

        ~Statsig()
        {
            Dispose(false);
        }

        public Task Initialize()
        {
            var source = new TaskCompletionSource<bool>();
            GCHandle handle = default;

            StatsigFFI.statsig_initialize_callback_delegate callback = () =>
            {
                try
                {
                    source.SetResult(true);
                }
                finally
                {
                    if (handle.IsAllocated)
                    {
                        handle.Free();
                    }
                }
            };

            handle = GCHandle.Alloc(callback);
            StatsigFFI.statsig_initialize(_statsigRef, callback);
            return source.Task;
        }

        unsafe public bool CheckGate(StatsigUser user, string gateName, EvaluationOptions? options = null)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                return StatsigFFI.statsig_check_gate(_statsigRef, user.Reference, gateNamePtr, optionsPtr);
            }
        }

        unsafe public FeatureGate GetFeatureGate(StatsigUser user, string gateName, EvaluationOptions? options = null)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_feature_gate(_statsigRef, user.Reference, gateNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                return jsonString != null
                    ? new FeatureGate(jsonString)
                    : new FeatureGate(string.Empty);
            }
        }

        unsafe public void ManuallyLogGateExposure(StatsigUser user, string gateName)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);

            fixed (byte* gateNamePtr = gateNameBytes)
            {
                StatsigFFI.statsig_manually_log_gate_exposure(_statsigRef, user.Reference, gateNamePtr);
            }
        }

        unsafe public DynamicConfig GetDynamicConfig(StatsigUser user, string configName, EvaluationOptions? options = null)
        {
            var configNameBytes = Encoding.UTF8.GetBytes(configName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* configNamePtr = configNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_dynamic_config(_statsigRef, user.Reference, configNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new DynamicConfig(string.Empty);
                }
                return jsonString != null
                    ? new DynamicConfig(jsonString)
                    : new DynamicConfig(string.Empty);
            }
        }

        unsafe public void ManuallyLogDynamicConfigExposure(StatsigUser user, string configName)
        {
            var configNameBytes = Encoding.UTF8.GetBytes(configName);

            fixed (byte* configNamePtr = configNameBytes)
            {
                StatsigFFI.statsig_manually_log_dynamic_config_exposure(_statsigRef, user.Reference, configNamePtr);
            }
        }

        unsafe public Experiment GetExperiment(StatsigUser user, string experimentName, EvaluationOptions? options = null)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_experiment(_statsigRef, user.Reference, experimentNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new Experiment(string.Empty);
                }
                return jsonString != null
                    ? new Experiment(jsonString)
                    : new Experiment(string.Empty);
            }
        }

        unsafe public void ManuallyLogExperimentExposure(StatsigUser user, string experimentName)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);

            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                StatsigFFI.statsig_manually_log_experiment_exposure(_statsigRef, user.Reference, experimentNamePtr);
            }
        }

        unsafe public Layer GetLayer(StatsigUser user, string layerName, EvaluationOptions? options = null)
        {
            var layerNameBytes = Encoding.UTF8.GetBytes(layerName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* layerNamePtr = layerNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_layer(_statsigRef, user.Reference, layerNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                return jsonString != null
                    ? new Layer(jsonString, _statsigRef, options)
                    : new Layer(string.Empty, _statsigRef, options);
            }
        }

        unsafe public void ManuallyLogLayerParameterExposure(StatsigUser user, string layerName, string parameterName)
        {
            var layerNameBytes = Encoding.UTF8.GetBytes(layerName);
            var parameterNameBytes = Encoding.UTF8.GetBytes(parameterName);
            fixed (byte* parameterNamePtr = parameterNameBytes)
            fixed (byte* layerNamePtr = layerNameBytes)
            {
                StatsigFFI.statsig_manually_log_layer_parameter_exposure(_statsigRef, user.Reference, layerNamePtr, parameterNamePtr);
            }
        }

        unsafe public ParameterStore GetParameterStore(StatsigUser user, string storeName, EvaluationOptions? options = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(storeName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* storeNamePtr = storeNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_parameter_store_with_options(_statsigRef, storeNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                return jsonString != null
                    ? new ParameterStore(jsonString, _statsigRef, user.Reference, options)
                    : new ParameterStore(string.Empty, _statsigRef, user.Reference, options);
            }
        }

        unsafe public string GetClientInitializeResponse(StatsigUser user, ClientInitResponseOptions? options = null)
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Failed to get statsig ref");
            }

            if (user.Reference == 0)
            {
                Console.WriteLine("Failed to get user reference");
            }

            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;

            fixed (byte* optionsPtr = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null)
            {
                var resPtr = StatsigFFI.statsig_get_client_init_response(_statsigRef, user.Reference, optionsPtr);
                return StatsigUtils.ReadStringFromPointer(resPtr) ?? string.Empty;
            }
        }

        unsafe public void OverrideGate(string gateName, bool value, string? id = null)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                StatsigFFI.statsig_override_gate(_statsigRef, gateNamePtr, value, idPtr);
            }
        }

        unsafe public void OverrideDynamicConfig(string configName, Dictionary<string, object> value, string? id = null)
        {
            var configNameBytes = Encoding.UTF8.GetBytes(configName);
            var jsonBytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(value));
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* configNamePtr = configNameBytes)
            fixed (byte* jsonPtr = jsonBytes)
            {
                StatsigFFI.statsig_override_dynamic_config(_statsigRef, configNamePtr, jsonPtr, idPtr);
            }
        }

        unsafe public void OverrideExperiment(string experimentName, Dictionary<string, object> value, string? id = null)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);
            var jsonBytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(value));
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* experimentNamePtr = experimentNameBytes)
            fixed (byte* jsonPtr = jsonBytes)
            {
                StatsigFFI.statsig_override_experiment(_statsigRef, experimentNamePtr, jsonPtr, idPtr);
            }
        }

        unsafe public void OverrideExperimentByGroupName(string experimentName, string groupName, string? id = null)
        {
            var groupNameBytes = Encoding.UTF8.GetBytes(groupName);
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* groupNamePtr = groupNameBytes)
            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                StatsigFFI.statsig_override_experiment_by_group_name(_statsigRef, experimentNamePtr, groupNamePtr, idPtr);
            }
        }

        unsafe public void OverrideLayer(string layerName, Dictionary<string, object> value, string? id = null)
        {
            var layerNameBytes = Encoding.UTF8.GetBytes(layerName);
            var jsonBytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(value));
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* layerNamePtr = layerNameBytes)
            fixed (byte* jsonPtr = jsonBytes)
            {
                StatsigFFI.statsig_override_layer(_statsigRef, layerNamePtr, jsonPtr, idPtr);
            }
        }

        public void LogEvent(StatsigUser user, string eventName, string? value = null, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }
        public void LogEvent(StatsigUser user, string eventName, int value, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }
        public void LogEvent(StatsigUser user, string eventName, double value, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }
        private unsafe void LogEventInternal(StatsigUser user, string eventName, object? value, IReadOnlyDictionary<string, string>? metadata)
        {
            var statsigEvent = new StatsigEvent(eventName, value, metadata);
            var eventJson = JsonConvert.SerializeObject(statsigEvent);
            var eventBytes = Encoding.UTF8.GetBytes(eventJson);
            fixed (byte* eventPtr = eventBytes)
            {
                StatsigFFI.statsig_log_event(_statsigRef, user.Reference, eventPtr);
            }
        }

        public void Identify(StatsigUser user)
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Statsig is not initialized.");
                return;
            }
            StatsigFFI.statsig_identify(_statsigRef, user.Reference);
        }

        public Task FlushEvents()
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Statsig is not initialized.");
                return Task.CompletedTask;
            }
            var source = new TaskCompletionSource<bool>();
            GCHandle handle = default;

            StatsigFFI.statsig_flush_events_callback_delegate callback = () =>
            {
                try
                {
                    source.SetResult(true);
                }
                finally
                {
                    if (handle.IsAllocated)
                    {
                        handle.Free();
                    }
                }
            };

            handle = GCHandle.Alloc(callback);
            StatsigFFI.statsig_flush_events(_statsigRef, callback);
            return source.Task;
        }

        public Task Shutdown()
        {
            var source = new TaskCompletionSource<bool>();
            GCHandle handle = default;

            StatsigFFI.statsig_shutdown_callback_delegate callback = () =>
            {
                try
                {
                    source.SetResult(true);
                }
                finally
                {
                    if (handle.IsAllocated)
                    {
                        handle.Free();
                    }
                }
            };

            handle = GCHandle.Alloc(callback);
            StatsigFFI.statsig_shutdown(_statsigRef, callback);
            return source.Task;
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (_statsigRef == 0)
            {
                return;
            }
            StatsigFFI.statsig_release(_statsigRef);
        }

        private unsafe void UpdateStatsigMetadata()
        {
            var sdkType = "statsig-server-core-dotnet";
            var os = RuntimeInformation.OSDescription;
            var arch = RuntimeInformation.OSArchitecture.ToString();
            var sdkVersion = Assembly
                .GetExecutingAssembly()
                .GetName()
                .Version?
                .ToString() ?? "unknown";

            var sdkTypeBytes = StatsigUtils.ToUtf8NullTerminated(sdkType);
            var osBytes = StatsigUtils.ToUtf8NullTerminated(os);
            var archBytes = StatsigUtils.ToUtf8NullTerminated(arch);
            var versionBytes = StatsigUtils.ToUtf8NullTerminated(sdkVersion);

            fixed (byte* sdkTypePtr = sdkTypeBytes)
            fixed (byte* osPtr = osBytes)
            fixed (byte* archPtr = archBytes)
            fixed (byte* versionPtr = versionBytes)
            {
                StatsigFFI.statsig_metadata_update_values(sdkTypePtr, osPtr, archPtr, versionPtr);
            }
        }
    }
}
