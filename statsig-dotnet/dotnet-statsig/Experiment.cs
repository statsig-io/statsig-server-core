using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class Experiment
    {
        [JsonProperty("name")] public string Name;
        [JsonProperty("rule_id")] public string RuleId;
        [JsonProperty("value")] public IReadOnlyDictionary<string, JToken> Value;
        [JsonProperty("group_name")] public string GroupName;
        [JsonProperty("details")] public EvaluationDetails Details;

        internal Experiment(string name, string ruleId, IReadOnlyDictionary<string, JToken> value, string groupName, EvaluationDetails details)
        {
            Name = name;
            RuleId = ruleId;
            Value = value;
            GroupName = groupName;
            Details = details;
        }

        // empty constructor when deserializing objects
        internal Experiment()
        {
        }
    }
}