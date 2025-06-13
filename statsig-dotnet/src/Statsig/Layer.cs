using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class Layer
    {
        [JsonProperty("name")] public string Name { get; }
        [JsonProperty("rule_id")] public string RuleID = "";
        [JsonProperty("__value")] internal IReadOnlyDictionary<string, JToken> Value { get; } // TODO this defined as internal in old dotnet
        [JsonProperty("group_name")] public string GroupName = "";
        [JsonProperty("id_type")] public string IDType = "";
        [JsonProperty("allocated_experiment_name")] public string AllocatedExperimentName = "";
        [JsonProperty("details")] public EvaluationDetails? EvaluationDetails { get; }

        private ulong _statsigRef = 0;
        private string rawJson;

        internal Layer(string rawJson, ulong statsigRef)
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
        }

        unsafe public T? Get<T>(string key, T? defaultValue = default(T))
        {
            JToken? outVal;
            if (!this.Value.TryGetValue(key, out outVal))
            {
                return defaultValue;
            }

            try
            {
                var result = outVal.ToObject<T>();
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