using System;
using System.Text;
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
#pragma warning disable IDE0052 // Remove unread private members
        [JsonProperty("name")]
        private readonly string Name;
        [JsonProperty("value")]
        private readonly object? Value;
        [JsonProperty("metadata")]
        private readonly IReadOnlyDictionary<string, string>? Metadata;
#pragma warning restore IDE0052

        internal StatsigEvent(string name, object? value = null, IReadOnlyDictionary<string, string>? metadata = null)
        {
            Name = name;
            Value = value;
            Metadata = metadata;
        }
    }
}