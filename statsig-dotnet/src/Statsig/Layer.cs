using System.Collections.Generic;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface ILayer
    {
        string Name { get; }
        EvaluationDetails? EvaluationDetails { get; }
        unsafe T? Get<T>(string key, T? defaultValue = default);
    }

    public class Layer : ILayer
    {
        [JsonProperty("name")] public string Name { get; }
        [JsonProperty("rule_id")] public string RuleID = "";
        [JsonProperty("__value")] internal IReadOnlyDictionary<string, JToken> Value { get; } // TODO this defined as internal in old dotnet
        [JsonProperty("group_name")] public string GroupName = "";
        [JsonProperty("id_type")] public string IDType = "";
        [JsonProperty("allocated_experiment_name")] public string AllocatedExperimentName = "";
        [JsonProperty("details")] public EvaluationDetails? EvaluationDetails { get; }

        private readonly ulong _statsigRef = 0;
        private readonly string rawJson;

        private readonly bool disableExposureLogging = false;

        internal Layer(string rawJson, ulong statsigRef, EvaluationOptions? options = null)
        {
            _statsigRef = statsigRef;
            this.rawJson = rawJson;
            var jsonObject = JObject.Parse(rawJson);
            Name = jsonObject["name"]?.ToString() ?? string.Empty;
            RuleID = jsonObject["rule_id"]?.ToString() ?? string.Empty;
            Value = jsonObject["__value"]?.ToObject<Dictionary<string, JToken>>() ?? new Dictionary<string, JToken>();
            GroupName = jsonObject["group_name"]?.ToString() ?? string.Empty;
            IDType = jsonObject["id_type"]?.ToString() ?? string.Empty;
            AllocatedExperimentName = jsonObject["allocated_experiment_name"]?.ToString() ?? string.Empty;
            EvaluationDetails = jsonObject["details"]?.ToObject<EvaluationDetails>();
            if (options != null)
            {
                disableExposureLogging = options.DisableExposureLogging;
            }
        }

        unsafe public T? Get<T>(string key, T? defaultValue = default)
        {
            if (!this.Value.TryGetValue(key, out JToken? outVal))
            {
                return defaultValue;
            }

            try
            {
                var result = outVal.ToObject<T>();

                if (disableExposureLogging)
                {
                    return result;
                }
                var layerJsonBytes = Encoding.UTF8.GetBytes(this.rawJson);
                var paramNameBytes = Encoding.UTF8.GetBytes(key);

                fixed (byte* layerJsonPointer = layerJsonBytes)
                fixed (byte* paramNamePointer = paramNameBytes)
                {
                    StatsigFFI.statsig_log_layer_param_exposure(this._statsigRef, layerJsonPointer, paramNamePointer);
                }
                return result;
            }
            catch
            {
                // There are a bunch of different types of exceptions that could
                // be thrown at this point - missing converters, format exception
                // type cast exception, etc.
                return defaultValue;
            }
        }
    }
}