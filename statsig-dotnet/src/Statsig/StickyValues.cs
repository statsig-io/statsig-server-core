using System;
using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Statsig
{
    public class StickyValues
    {
        [JsonProperty("value")]
        public bool Value { get; set; }

        [JsonProperty("json_value")]
        public JObject? JsonValue { get; set; }

        [JsonProperty("rule_id")]
        public string? RuleId { get; set; }

        [JsonProperty("group_name")]
        public string? GroupName { get; set; }

        [JsonProperty("secondary_exposures")]
        public IList<SecondaryExposure>? SecondaryExposures { get; set; } = new List<SecondaryExposure>();

        [JsonProperty("undelegated_secondary_exposures")]
        public IList<SecondaryExposure>? UndelegatedSecondaryExposures { get; set; } = new List<SecondaryExposure>();

        [JsonProperty("config_delegate")]
        public string? ConfigDelegate { get; set; }

        [JsonProperty("explicit_parameters")]
        public IList<string>? ExplicitParameters { get; set; }

        [JsonProperty("time")]
        public long? Time { get; set; }

        [JsonProperty("config_version")]
        public uint? ConfigVersion { get; set; }
    }

    public sealed class SecondaryExposure
    {
        [JsonProperty("gate")]
        public string? Gate { get; set; }

        [JsonProperty("gateValue")]
        public string? GateValue { get; set; }

        [JsonProperty("ruleID")]
        public string? RuleId { get; set; }
    }
}