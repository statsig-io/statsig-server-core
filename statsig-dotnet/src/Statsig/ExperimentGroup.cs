using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface IExperimentGroup
    {
        string GroupName { get; }
        string RuleID { get; }
        string IDType { get; }
        IReadOnlyDictionary<string, JToken> ReturnValue { get; }
    }

    public class ExperimentGroup : IExperimentGroup
    {
        [JsonProperty("group_name")] public string GroupName { get; }
        [JsonProperty("rule_id")] public string RuleID { get; }
        [JsonProperty("id_type")] public string IDType { get; }
        [JsonProperty("return_value")] public IReadOnlyDictionary<string, JToken> ReturnValue { get; }

        internal ExperimentGroup(JToken token)
        {
            GroupName = token["group_name"]?.ToString() ?? string.Empty;
            RuleID = token["rule_id"]?.ToString() ?? string.Empty;
            IDType = token["id_type"]?.ToString() ?? string.Empty;
            ReturnValue = token["return_value"]?.ToObject<Dictionary<string, JToken>>()
                ?? new Dictionary<string, JToken>();
        }
    }
}
