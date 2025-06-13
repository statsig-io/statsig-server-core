using System;
using System.Text;
using System.Text.Json;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json;
using System.Collections.Generic;

namespace Statsig
{
    /// <summary>
    ///  Represents an internal event for logging within the Statsig SDK.
    /// </summary>
    internal class StatsigEvent
    {
        [JsonProperty("name")]
        private string Name;
        [JsonProperty("value")]
        private object? Value;
        [JsonProperty("metadata")]
        private IReadOnlyDictionary<string, string>? Metadata;

        internal StatsigEvent(string name, object? value = null, IReadOnlyDictionary<string, string>? metadata = null)
        {
            Name = name;
            Value = value;
            Metadata = metadata;
        }
    }
}