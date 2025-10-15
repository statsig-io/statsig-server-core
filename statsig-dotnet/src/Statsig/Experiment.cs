using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface IExperiment
    {
        string Name { get; }
        string RuleID { get; }
        IReadOnlyDictionary<string, JToken> Value { get; }
        string? GroupName { get; }
        EvaluationDetails? EvaluationDetails { get; }
        string? IDType { get; }
        T? Get<T>(string key, T? defaultValue = default);
    }

    public class Experiment : IExperiment
    {
        [JsonProperty("name")] public string Name { get; }
        [JsonProperty("rule_id")] public string RuleID { get; }
        [JsonProperty("value")] public IReadOnlyDictionary<string, JToken> Value { get; }
        [JsonProperty("group_name")] public string? GroupName { get; }
        [JsonProperty("details")] public EvaluationDetails? EvaluationDetails { get; }
        [JsonProperty("id_type")] public string? IDType { get; }

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