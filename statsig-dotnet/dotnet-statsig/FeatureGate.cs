using Newtonsoft.Json.Linq;
using Newtonsoft.Json;

namespace Statsig
{
    public class FeatureGate
    {
        [JsonProperty("name")]
        public string Name { get; }
        
        [JsonProperty("value")]
        public bool EvalValue { get; }
        
        [JsonProperty("rule_id")]
        public string RuleId { get; }
        
        [JsonProperty("details")]
        public EvaluationDetails EvaluationDetails { get; }
        public string RawJson { get; set; }

        internal FeatureGate(string name, bool evalValue, string ruleID, EvaluationDetails evaluationDetails, string rawJson)
        {
            Name = name;
            EvalValue = evalValue;
            RuleId = ruleID;
            EvaluationDetails = evaluationDetails;
            RawJson = rawJson;
        }
    }
}