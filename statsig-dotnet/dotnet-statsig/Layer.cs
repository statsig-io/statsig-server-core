using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class Layer
    {
        [JsonProperty("name")] public string Name { get; }
        [JsonProperty("rule_id")] public string RuleId { get; }
        [JsonProperty("__value")] public  IReadOnlyDictionary<string, JToken> Value { get; } // TODO this defined as internal in old dotnet
        [JsonProperty("group_name")] public string GroupName { get; }
        [JsonProperty("allocated_experiment_name")] public string AllocatedExperimentName { get; }
        [JsonProperty("details")] public EvaluationDetails EvaluationDetails { get; }

        internal Layer(string name, string ruleId, IReadOnlyDictionary<string, JToken> value, string groupName, string allocatedExperimentName,
            EvaluationDetails evaluationDetails)
        {
            Name = name;
            RuleId = ruleId;
            Value = value;
            GroupName = groupName;
            AllocatedExperimentName = allocatedExperimentName;
            EvaluationDetails = evaluationDetails;
        }
    }
}