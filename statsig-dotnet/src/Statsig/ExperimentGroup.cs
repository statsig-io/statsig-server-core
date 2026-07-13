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

    public interface IExperimentGroupsResult
    {
        /// <summary>
        /// Null when the name does not refer to an experiment (unknown name or a dynamic
        /// config); otherwise the experiment's isActive state.
        /// </summary>
        bool? IsExperimentActive { get; }
        IReadOnlyList<IExperimentGroup> Groups { get; }
    }

    public class ExperimentGroupsResult : IExperimentGroupsResult
    {
        [JsonProperty("is_experiment_active")] public bool? IsExperimentActive { get; }
        [JsonProperty("groups")] public IReadOnlyList<IExperimentGroup> Groups { get; }

        internal ExperimentGroupsResult()
        {
            IsExperimentActive = null;
            Groups = new List<IExperimentGroup>();
        }

        internal ExperimentGroupsResult(JObject jsonObject)
        {
            IsExperimentActive = jsonObject["is_experiment_active"]?.Type == JTokenType.Boolean
                ? jsonObject["is_experiment_active"]?.ToObject<bool>()
                : null;

            var groups = new List<IExperimentGroup>();
            if (jsonObject["groups"] is JArray array)
            {
                foreach (var token in array)
                {
                    groups.Add(new ExperimentGroup(token));
                }
            }
            Groups = groups;
        }
    }
}
