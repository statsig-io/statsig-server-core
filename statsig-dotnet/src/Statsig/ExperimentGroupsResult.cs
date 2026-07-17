using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface IExperimentGroupsResult
    {
        /// <summary>
        /// Null when the name does not refer to an experiment (unknown name or a
        /// non-experiment entity like a dynamic config or autotune); otherwise the
        /// experiment's isActive state.
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
                    if (token is JObject groupObject)
                    {
                        groups.Add(new ExperimentGroup(groupObject));
                    }
                }
            }
            Groups = groups;
        }
    }
}
