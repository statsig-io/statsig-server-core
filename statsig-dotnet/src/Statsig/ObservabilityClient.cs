using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IObservabilityClient
    {
        void Init();
        void Increment(string metric, double value, Dictionary<string, string>? tags);
        void Gauge(string metric, double value, Dictionary<string, string>? tags);
        void Dist(string metric, double value, Dictionary<string, string>? tags);
        void Error(string tag, string error);
        bool ShouldEnableHighCardinalityForThisTag(string tag);
    }

    public abstract class ObservabilityClient : IObservabilityClient, IDisposable
    {
        private readonly StatsigFFI.observability_client_init_fn_delegate _initDelegate;
        private readonly StatsigFFI.observability_client_increment_fn_delegate _incrementDelegate;
        private readonly StatsigFFI.observability_client_gauge_fn_delegate _gaugeDelegate;
        private readonly StatsigFFI.observability_client_dist_fn_delegate _distDelegate;
        private readonly StatsigFFI.observability_client_error_fn_delegate _errorDelegate;
        private readonly StatsigFFI.observability_client_should_enable_high_cardinality_for_this_tag_fn_delegate _shouldEnableHighCardinalityDelegate;

        private bool _disposed;

        internal ulong Reference { get; private set; }

        public unsafe ObservabilityClient()
        {
            _initDelegate = InitNative;
            _incrementDelegate = IncrementNative;
            _gaugeDelegate = GaugeNative;
            _distDelegate = DistNative;
            _errorDelegate = ErrorNative;
            _shouldEnableHighCardinalityDelegate = ShouldEnableHighCardinalityNative;

            Reference = StatsigFFI.observability_client_create(
                _initDelegate,
                _incrementDelegate,
                _gaugeDelegate,
                _distDelegate,
                _errorDelegate,
                _shouldEnableHighCardinalityDelegate);

            if (Reference == 0)
            {
                Console.Error.WriteLine("[Statsig] Failed to register observability client with the native bridge.");
            }
        }

        ~ObservabilityClient()
        {
            Dispose(false);
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (_disposed)
            {
                return;
            }

            if (Reference != 0)
            {
                StatsigFFI.observability_client_release(Reference);
                Reference = 0;
            }

            _disposed = true;
        }

        public virtual void Init()
        {
        }

        public virtual void Increment(string metric, double value, Dictionary<string, string>? tags)
        {
        }

        public virtual void Gauge(string metric, double value, Dictionary<string, string>? tags)
        {
        }

        public virtual void Dist(string metric, double value, Dictionary<string, string>? tags)
        {
        }

        public virtual void Error(string tag, string error)
        {
        }

        public virtual bool ShouldEnableHighCardinalityForThisTag(string tag)
        {
            return false;
        }

        private unsafe void InitNative()
        {
            try
            {
                Init();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] ObservabilityClient.Init failed: {ex}");
            }
        }

        private unsafe void IncrementNative(byte* argsPtr, ulong argsLength)
        {
            HandleMetricCallback(argsPtr, argsLength, Increment, nameof(Increment));
        }

        private unsafe void GaugeNative(byte* argsPtr, ulong argsLength)
        {
            HandleMetricCallback(argsPtr, argsLength, Gauge, nameof(Gauge));
        }

        private unsafe void DistNative(byte* argsPtr, ulong argsLength)
        {
            HandleMetricCallback(argsPtr, argsLength, Dist, nameof(Dist));
        }

        private static unsafe void HandleMetricCallback(
            byte* argsPtr,
            ulong argsLength,
            Action<string, double, Dictionary<string, string>?> handler,
            string name)
        {
            try
            {
                var args = DeserializeFromPointer<MetricArgs>(argsPtr, argsLength);
                if (args == null || args.Metric == null)
                {
                    return;
                }

                handler(args.Metric, args.Value, args.Tags);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] ObservabilityClient.{name} failed: {ex}");
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private unsafe void ErrorNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                var args = DeserializeFromPointer<ErrorArgs>(argsPtr, argsLength);
                if (args == null || args.Tag == null || args.Error == null)
                {
                    return;
                }

                Error(args.Tag, args.Error);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] ObservabilityClient.Error failed: {ex}");
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private unsafe bool ShouldEnableHighCardinalityNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                // The core passes the tag as a raw (non-JSON) string here.
                var tag = StatsigUtils.Utf8PointerToString(argsPtr, argsLength);
                if (string.IsNullOrEmpty(tag))
                {
                    return false;
                }

                return ShouldEnableHighCardinalityForThisTag(tag!);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] ObservabilityClient.ShouldEnableHighCardinalityForThisTag failed: {ex}");
                return false;
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private static unsafe T? DeserializeFromPointer<T>(byte* ptr, ulong length)
        {
            var json = StatsigUtils.Utf8PointerToString(ptr, length);
            if (string.IsNullOrEmpty(json))
            {
                return default;
            }

            return JsonConvert.DeserializeObject<T?>(json);
        }

        private sealed class MetricArgs
        {
            [JsonProperty("metric")] public string? Metric { get; set; }
            [JsonProperty("value")] public double Value { get; set; }
            [JsonProperty("tags")] public Dictionary<string, string>? Tags { get; set; }
        }

        private sealed class ErrorArgs
        {
            [JsonProperty("tag")] public string? Tag { get; set; }
            [JsonProperty("error")] public string? Error { get; set; }
        }
    }
}
