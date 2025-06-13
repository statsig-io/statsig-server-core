using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using System.Collections.Generic;

namespace Statsig
{
    public class Statsig : IDisposable
    {
        private unsafe ulong _statsigRef;

        public Statsig(string sdkKey, StatsigOptions options)
        {
            Console.WriteLine($"Operating System: {RuntimeInformation.OSDescription}");
            Console.WriteLine($"Architecture: {RuntimeInformation.OSArchitecture}");
            var sdkKeyBytes = Encoding.UTF8.GetBytes(sdkKey);
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
            StatsigFFI.statsig_initialize(_statsigRef, () =>
            {
                source.SetResult(true);
            });
            return source.Task;
        }

        unsafe public bool CheckGate(StatsigUser user, string gateName)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                return StatsigFFI.statsig_check_gate(_statsigRef, user.Reference, gateNamePtr, null);
            }
        }

        unsafe public DynamicConfig GetConfig(StatsigUser user, string configName)
        {
            var configNameBytes = Encoding.UTF8.GetBytes(configName);

            fixed (byte* configNamePtr = configNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_dynamic_config(_statsigRef, user.Reference, configNamePtr, null);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new DynamicConfig(configName, null, null, null);
                }
                return JsonConvert.DeserializeObject<DynamicConfig>(jsonString) ??
                       new DynamicConfig(configName, null, null, null);
            }
        }

        unsafe public Experiment GetExperiment(StatsigUser user, string experimentName)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);

            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_experiment(_statsigRef, user.Reference, experimentNamePtr, null);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new Experiment(experimentName, null, null, null, null);
                }
                return JsonConvert.DeserializeObject<Experiment>(jsonString) ??
                       new Experiment(experimentName, null, null, null, null);
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

        public void FlushEvents()
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Statsig is not initialized.");
                return;
            }
            StatsigFFI.statsig_flush_events_blocking(_statsigRef);
        }

        public void Shutdown()
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Statsig is not initialized.");
                return;
            }
            StatsigFFI.statsig_shutdown_blocking(_statsigRef);
            this.Dispose();
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
    }
}