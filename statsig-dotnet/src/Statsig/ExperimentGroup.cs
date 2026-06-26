using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public interface IExperimentGroup
    {
        string GroupName { get; }
        IReadOnlyDictionary<string, JToken> ReturnValue { get; }
    }

    public class ExperimentGroup : IExperimentGroup
    {
        [JsonProperty("group_name")] public string GroupName { get; }
        [JsonProperty("return_value")] public IReadOnlyDictionary<string, JToken> ReturnValue { get; }

        internal ExperimentGroup(JToken token)
        {
            GroupName = token["group_name"]?.ToString() ?? string.Empty;
            ReturnValue = token["return_value"]?.ToObject<Dictionary<string, JToken>>()
                ?? new Dictionary<string, JToken>();
        }
    }
}
