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
            if (string.IsNullOrEmpty(rawJson))
            {
                Name = string.Empty;
                RuleID = string.Empty;
                Value = new Dictionary<string, JToken>();
                GroupName = null;
                IDType = string.Empty;
                EvaluationDetails = null;
                return;
            }

            var jsonObject = JObject.Parse(rawJson);
            Name = jsonObject["name"]?.ToString() ?? string.Empty;
            // The normal GetExperiment path returns the snake_case shape, while
            // the group-targeting getters return the camelCase ExperimentRaw
            // shape (ruleID / idType / groupName). Accept both key styles.
            RuleID = (jsonObject["rule_id"] ?? jsonObject["ruleID"])?.ToString() ?? string.Empty;
            Value = jsonObject["value"]?.ToObject<Dictionary<string, JToken>>() ?? new Dictionary<string, JToken>();
            var groupNameToken = jsonObject["group_name"] ?? jsonObject["groupName"];
            GroupName = (groupNameToken == null || groupNameToken.Type == JTokenType.Null)
                ? null
                : groupNameToken.ToString();
            IDType = (jsonObject["id_type"] ?? jsonObject["idType"])?.ToString() ?? string.Empty;
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