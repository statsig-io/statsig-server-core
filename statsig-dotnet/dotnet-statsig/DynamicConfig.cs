using System.Collections.Generic;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class DynamicConfig : Experiment
    {
        
        internal DynamicConfig(string name, string ruleId, IReadOnlyDictionary<string, JToken> value, EvaluationDetails details)
            : base(name, ruleId, value, null, details)  // Passing null for GroupName
        {
        }
    }
}