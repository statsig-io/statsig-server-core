using Newtonsoft.Json.Linq;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IFeatureGate
    {
        string Name { get; }
        bool Value { get; }
        string RuleID { get; }
        string? IDType { get; }
        EvaluationDetails? EvaluationDetails { get; }
    }

    public class FeatureGate : IFeatureGate
    {
        [JsonProperty("name")]
        public string Name { get; }

        [JsonProperty("value")]
        public bool Value { get; }

        [JsonProperty("rule_id")]
        public string RuleID { get; }

        [JsonProperty("id_type")]
        public string? IDType { get; }

        [JsonProperty("details")]
        public EvaluationDetails? EvaluationDetails { get; }

        internal FeatureGate(string rawJson)
        {
            var jsonObject = JObject.Parse(rawJson);
            Name = jsonObject["name"]?.ToString() ?? string.Empty;
            Value = jsonObject["value"]?.ToObject<bool>() ?? false;
            RuleID = jsonObject["rule_id"]?.ToString() ?? string.Empty;
            EvaluationDetails = jsonObject["details"]?.ToObject<EvaluationDetails>();
            IDType = jsonObject["id_type"]?.ToString();
        }
    }
}