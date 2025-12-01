using System.Collections.Generic;
using Newtonsoft.Json;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the GetClientInitializeResponse method in the Statsig Server SDK.
    /// </summary>
    public class EvaluationOptions
    {
        [JsonProperty("disable_exposure_logging")]
        public bool DisableExposureLogging { get; set; }

        [JsonProperty("user_persisted_values", NullValueHandling = NullValueHandling.Ignore)]
        public Dictionary<string, StickyValues>? UserPersistedValues { get; set; }

        public EvaluationOptions(bool disableExposureLogging = false, Dictionary<string, StickyValues>? userPersistedValues = null)
        {
            DisableExposureLogging = disableExposureLogging;
            UserPersistedValues = userPersistedValues;
        }
    }
}
