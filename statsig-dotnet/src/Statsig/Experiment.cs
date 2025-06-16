using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class Experiment
    {
        [JsonProperty("name")] public string Name;
        [JsonProperty("rule_id")] public string RuleID;
        [JsonProperty("value")] public IReadOnlyDictionary<string, JToken> Value;
        [JsonProperty("group_name")] public string? GroupName;
        [JsonProperty("details")] public EvaluationDetails? EvaluationDetails;
        [JsonProperty("id_type")] public string? IDType;

        internal Experiment(string rawJson)
        {
            var jsonObject = JObject.Parse(rawJson);
            Name = jsonObject["name"]?.ToString() ?? string.Empty;
            RuleID = jsonObject["rule_id"]?.ToString() ?? string.Empty;
            Value = jsonObject["value"]?.ToObject<Dictionary<string, JToken>>() ?? new Dictionary<string, JToken>();
            GroupName = jsonObject["group_name"]?.ToString();
            IDType = jsonObject["id_type"]?.ToString() ?? string.Empty;
            EvaluationDetails = jsonObject["details"]?.ToObject<EvaluationDetails>();
        }

        public T? Get<T>(string key, T? defaultValue = default)
        {
            if (!this.Value.TryGetValue(key, out JToken? outVal))
            {
                return defaultValue;
            }

            try
            {
                var result = outVal.ToObject<T>();
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