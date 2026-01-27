using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Reflection;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IStatsig
    {
        Task Initialize();
        bool CheckGate(IStatsigUser user, string gateName, EvaluationOptions? options = null);
        IFeatureGate GetFeatureGate(IStatsigUser user, string gateName, EvaluationOptions? options = null);
        void ManuallyLogGateExposure(IStatsigUser user, string gateName);
        IDynamicConfig GetDynamicConfig(IStatsigUser user, string configName, EvaluationOptions? options = null);
        void ManuallyLogDynamicConfigExposure(IStatsigUser user, string configName);
        IExperiment GetExperiment(IStatsigUser user, string experimentName, EvaluationOptions? options = null);
        void ManuallyLogExperimentExposure(IStatsigUser user, string experimentName);
        ILayer GetLayer(IStatsigUser user, string layerName, EvaluationOptions? options = null);
        void ManuallyLogLayerParameterExposure(IStatsigUser user, string layerName, string parameterName);
        IParameterStore GetParameterStore(IStatsigUser user, string storeName, EvaluationOptions? options = null);
        string GetClientInitializeResponse(IStatsigUser user, ClientInitResponseOptions? options = null);
        void LogEvent(IStatsigUser user, string eventName, string? value = null, IReadOnlyDictionary<string, string>? metadata = null);
        void LogEvent(IStatsigUser user, string eventName, int value, IReadOnlyDictionary<string, string>? metadata = null);
        void LogEvent(IStatsigUser user, string eventName, double value, IReadOnlyDictionary<string, string>? metadata = null);
        void Identify(IStatsigUser user);
        Task FlushEvents();
        Task Shutdown();
    }

    public class Statsig : IDisposable, IStatsig
    {
        private const int SpecNameStackThreshold = 256;
        private const int EvalOptStackThreshold = 512;
        private const int JsonStackThreshold = 1024;

        private readonly unsafe ulong _statsigRef;
        private readonly StatsigOptions _options;

        // Shared Instance
        private static Statsig? sharedInstance = null;
        private static readonly object lockObject = new();

        public static Statsig Shared()
        {
            if (!HasShared() || sharedInstance == null)
            {
                Console.Error.WriteLine(
                    "[Statsig] No shared instance found. Please call NewShared() before accessing the shared instance. Returning an invalid instance.");
                return CreateErrorStatsigInstance();
            }

            return sharedInstance;
        }

        public static bool HasShared()
        {
            return sharedInstance != null;
        }

        public static IStatsig NewShared(string sdkKey, StatsigOptions options)
        {
            lock (lockObject)
            {
                if (HasShared())
                {
                    Console.Error.WriteLine(
                        "[Statsig] Shared instance already exists. Call RemoveSharedInstance() before creating a new one. Returning an invalid instance.");
                    return CreateErrorStatsigInstance();
                }
                sharedInstance = new Statsig(sdkKey, options);
            }

            return sharedInstance;
        }

        public static IStatsig NewShared(string sdkKey)
        {
            lock (lockObject)
            {
                if (HasShared())
                {
                    Console.Error.WriteLine(
                        "[Statsig] Shared instance already exists. Call RemoveSharedInstance() before creating a new one. Returning an invalid instance.");
                    return CreateErrorStatsigInstance();
                }
                sharedInstance = new Statsig(sdkKey);
            }

            return sharedInstance;
        }

        public static void RemoveSharedInstance()
        {
            lock (lockObject)
            {
                sharedInstance = null;
            }
        }

        public Statsig(string sdkKey, StatsigOptions? options = null)
        {
            options ??= new StatsigOptions(new StatsigOptionsBuilder());
            _options = options;

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

        // --------------------------
        // Check Gate
        // --------------------------

        unsafe public bool CheckGate(IStatsigUser user, string gateName, EvaluationOptions? options = null)
        {
            // Get gate name bytes
            int nameLen = Encoding.UTF8.GetByteCount(gateName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen <= SpecNameStackThreshold ? stackalloc byte[nameLen] : new byte[nameLen];
            Encoding.UTF8.GetBytes(gateName.AsSpan(), nameBytes);
#else
            byte[] nameBytesArray = new byte[nameLen];
            Encoding.UTF8.GetBytes(gateName, 0, gateName.Length, nameBytesArray, 0);
            Span<byte> nameBytes = nameBytesArray;
#endif

            // Get options bytes
            string? optionsJson = options is null ? null : JsonConvert.SerializeObject(options);
            int optLen = optionsJson is null ? 0 : Encoding.UTF8.GetByteCount(optionsJson);
#if NET8_0_OR_GREATER
            Span<byte> optBytes = optLen <= EvalOptStackThreshold ? stackalloc byte[optLen] : new byte[optLen];
            if (optLen > 0)
            {
                Encoding.UTF8.GetBytes(optionsJson.AsSpan(), optBytes);
            }
#else
            byte[] optBytesArray = optLen > 0 ? new byte[optLen] : Array.Empty<byte>();
            if (optLen > 0)
            {
                Encoding.UTF8.GetBytes(optionsJson, 0, optionsJson.Length, optBytesArray, 0);
            }
            Span<byte> optBytes = optBytesArray;
#endif

            fixed (byte* namePtr = nameBytes)
            fixed (byte* optPtr = optBytes)
            {
                return StatsigFFI.statsig_check_gate_performance(
                    _statsigRef,
                    user.Reference,
                    namePtr,
                    (nuint)nameBytes.Length,
                    optPtr,
                    (nuint)optBytes.Length);
            }
        }

        unsafe public IFeatureGate GetFeatureGate(IStatsigUser user, string gateName, EvaluationOptions? options = null)
        {
            int nameLen = Encoding.UTF8.GetByteCount(gateName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(gateName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(gateName, 0, gateName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif

            string? optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            byte[]? optBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optBytes)
            fixed (byte* gateNamePtr = nameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_feature_gate(_statsigRef, user.Reference, gateNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                return jsonString != null
                    ? new FeatureGate(jsonString)
                    : new FeatureGate(string.Empty);
            }
        }

        unsafe public void ManuallyLogGateExposure(IStatsigUser user, string gateName)
        {
            int nameLen = Encoding.UTF8.GetByteCount(gateName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(gateName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(gateName, 0, gateName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif
            fixed (byte* gateNamePtr = nameBytes)
            {
                StatsigFFI.statsig_manually_log_gate_exposure(_statsigRef, user.Reference, gateNamePtr);
            }
        }

        // --------------------------
        // Get Dynamic Config
        // --------------------------

        unsafe public IDynamicConfig GetDynamicConfig(IStatsigUser user, string configName, EvaluationOptions? options = null)
        {
            int nameLen = Encoding.UTF8.GetByteCount(configName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(configName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(configName, 0, configName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif

            string? optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            byte[]? optBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optBytes)
            fixed (byte* configNamePtr = nameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_dynamic_config(_statsigRef, user.Reference, configNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new DynamicConfig(string.Empty);
                }
                return new DynamicConfig(jsonString);
            }
        }

        unsafe public void ManuallyLogDynamicConfigExposure(IStatsigUser user, string configName)
        {
            int nameLen = Encoding.UTF8.GetByteCount(configName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(configName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(configName, 0, configName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif
            fixed (byte* configNamePtr = nameBytes)
            {
                StatsigFFI.statsig_manually_log_dynamic_config_exposure(_statsigRef, user.Reference, configNamePtr);
            }
        }

        // --------------------------
        // Get Experiment
        // --------------------------

        unsafe public IExperiment GetExperiment(IStatsigUser user, string experimentName, EvaluationOptions? options = null)
        {
            int nameLen = Encoding.UTF8.GetByteCount(experimentName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(experimentName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(experimentName, 0, experimentName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif

            string? optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            byte[]? optBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optBytes)
            fixed (byte* experimentNamePtr = nameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_experiment(_statsigRef, user.Reference, experimentNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new Experiment(string.Empty);
                }
                return new Experiment(jsonString);
            }
        }

        unsafe public void ManuallyLogExperimentExposure(IStatsigUser user, string experimentName)
        {
            int nameLen = Encoding.UTF8.GetByteCount(experimentName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(experimentName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(experimentName, 0, experimentName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif
            fixed (byte* experimentNamePtr = nameBytes)
            {
                StatsigFFI.statsig_manually_log_experiment_exposure(_statsigRef, user.Reference, experimentNamePtr);
            }
        }

        // --------------------------
        // Get Layer
        // --------------------------

        unsafe public ILayer GetLayer(IStatsigUser user, string layerName, EvaluationOptions? options = null)
        {
            int nameLen = Encoding.UTF8.GetByteCount(layerName);
#if NET8_0_OR_GREATER
            Span<byte> nameBytes = nameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[nameLen + 1] : new byte[nameLen + 1];
            int written = Encoding.UTF8.GetBytes(layerName, nameBytes[..nameLen]);
            nameBytes[written] = 0;
#else
            byte[] nameBytesArray = new byte[nameLen + 1];
            Encoding.UTF8.GetBytes(layerName, 0, layerName.Length, nameBytesArray, 0);
            nameBytesArray[nameLen] = 0;
            Span<byte> nameBytes = nameBytesArray;
#endif

            string? optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            byte[]? optBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optBytes)
            fixed (byte* layerNamePtr = nameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_layer(_statsigRef, user.Reference, layerNamePtr, optionsPtr);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new Layer(string.Empty, _statsigRef, options);
                }
                return new Layer(jsonString, _statsigRef, options);
            }
        }


        // --------------------------
        // Parameter Store
        // --------------------------

        unsafe public void ManuallyLogLayerParameterExposure(IStatsigUser user, string layerName, string parameterName)
        {
            int layerNameLen = Encoding.UTF8.GetByteCount(layerName);
#if NET8_0_OR_GREATER
            Span<byte> layerNameBytes = layerNameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[layerNameLen + 1] : new byte[layerNameLen + 1];
            int writtenLayer = Encoding.UTF8.GetBytes(layerName, layerNameBytes[..layerNameLen]);
            layerNameBytes[writtenLayer] = 0;
#else
            byte[] layerNameBytesArray = new byte[layerNameLen + 1];
            Encoding.UTF8.GetBytes(layerName, 0, layerName.Length, layerNameBytesArray, 0);
            layerNameBytesArray[layerNameLen] = 0;
            Span<byte> layerNameBytes = layerNameBytesArray;
#endif
            int paramNameLen = Encoding.UTF8.GetByteCount(parameterName);
#if NET8_0_OR_GREATER
            Span<byte> paramNameBytes = paramNameLen + 1 <= SpecNameStackThreshold ? stackalloc byte[paramNameLen + 1] : new byte[paramNameLen + 1];
            int writtenParam = Encoding.UTF8.GetBytes(parameterName, paramNameBytes[..paramNameLen]);
            paramNameBytes[writtenParam] = 0;
#else
            byte[] paramNameBytesArray = new byte[paramNameLen + 1];
            Encoding.UTF8.GetBytes(parameterName, 0, parameterName.Length, paramNameBytesArray, 0);
            paramNameBytesArray[paramNameLen] = 0;
            Span<byte> paramNameBytes = paramNameBytesArray;
#endif
            fixed (byte* parameterNamePtr = paramNameBytes)
            fixed (byte* layerNamePtr = layerNameBytes)
            {
                StatsigFFI.statsig_manually_log_layer_parameter_exposure(_statsigRef, user.Reference, layerNamePtr, parameterNamePtr);
            }
        }

        unsafe public IParameterStore GetParameterStore(IStatsigUser user, string storeName, EvaluationOptions? options = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(storeName);
            var optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            var optionsBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;

            fixed (byte* optionsPtr = optionsBytes)
            fixed (byte* storeNamePtr = storeNameBytes)
            {
                ulong resultLen = 0;
                var jsonStringPtr = StatsigFFI.statsig_get_parameter_store_with_options(
                    _statsigRef,
                    storeNamePtr,
                    optionsPtr,
                    &resultLen
                );
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr, resultLen);
                return jsonString != null
                    ? new ParameterStore(jsonString, _statsigRef, user.Reference, options)
                    : new ParameterStore(string.Empty, _statsigRef, user.Reference, options);
            }
        }

        // --------------------------
        // Get Client Initialize Response
        // --------------------------

        unsafe public string GetClientInitializeResponse(IStatsigUser user, ClientInitResponseOptions? options = null)
        {
            string? optionsJson = options != null ? JsonConvert.SerializeObject(options) : null;
            byte[]? optBytes = optionsJson != null ? Encoding.UTF8.GetBytes(optionsJson) : null;
            fixed (byte* optionsPtr = optBytes)
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

        unsafe public void OverrideParameterStore(string paramName, Dictionary<string, object> value, string? id = null)
        {
            var paramNameBytes = Encoding.UTF8.GetBytes(paramName);
            var jsonBytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(value));
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* paramNamePtr = paramNameBytes)
            fixed (byte* jsonPtr = jsonBytes)
            {
                StatsigFFI.statsig_override_parameter_store(_statsigRef, paramNamePtr, jsonPtr, idPtr);
            }
        }

        unsafe public void RemoveGateOverride(string gateName, string? id = null)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                StatsigFFI.statsig_remove_gate_override(_statsigRef, gateNamePtr, idPtr);
            }
        }

        unsafe public void RemoveDynamicConfigOverride(string configName, string? id = null)
        {
            var configNameBytes = Encoding.UTF8.GetBytes(configName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* configNamePtr = configNameBytes)
            {
                StatsigFFI.statsig_remove_dynamic_config_override(_statsigRef, configNamePtr, idPtr);
            }
        }

        unsafe public void RemoveExperimentOverride(string experimentName, string? id = null)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                StatsigFFI.statsig_remove_experiment_override(_statsigRef, experimentNamePtr, idPtr);
            }
        }

        unsafe public void RemoveLayerOverride(string layerName, string? id = null)
        {
            var layerNameBytes = Encoding.UTF8.GetBytes(layerName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* layerNamePtr = layerNameBytes)
            {
                StatsigFFI.statsig_remove_layer_override(_statsigRef, layerNamePtr, idPtr);
            }
        }

        unsafe public void RemoveParameterStoreOverride(string storeName, string? id = null)
        {
            var storeNameBytes = Encoding.UTF8.GetBytes(storeName);
            var idBytes = id != null ? Encoding.UTF8.GetBytes(id) : null;
            fixed (byte* idPtr = idBytes)
            fixed (byte* storeNamePtr = storeNameBytes)
            {
                StatsigFFI.statsig_remove_parameter_store_override(_statsigRef, storeNamePtr, idPtr);
            }
        }

        public void RemoveAllOverrides()
        {
            StatsigFFI.statsig_remove_all_overrides(_statsigRef);
        }

        public void LogEvent(IStatsigUser user, string eventName, string? value = null, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }

        public void LogEvent(IStatsigUser user, string eventName, int value, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }

        public void LogEvent(IStatsigUser user, string eventName, double value, IReadOnlyDictionary<string, string>? metadata = null)
        {
            LogEventInternal(user, eventName, value, metadata);
        }

        private unsafe void LogEventInternal(IStatsigUser user, string eventName, object? value, IReadOnlyDictionary<string, string>? metadata)
        {
            var statsigEvent = new StatsigEvent(eventName, value, metadata);
            var eventJson = JsonConvert.SerializeObject(statsigEvent);
            var eventBytes = Encoding.UTF8.GetBytes(eventJson);
            fixed (byte* eventPtr = eventBytes)
            {
                StatsigFFI.statsig_log_event(_statsigRef, user.Reference, eventPtr);
            }
        }

        public void Identify(IStatsigUser user)
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

        private static Statsig CreateErrorStatsigInstance()
        {
            return new Statsig("Invalid SDK Key");
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
