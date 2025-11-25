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

        public EvaluationOptions(bool disableExposureLogging = false)
        {
            DisableExposureLogging = disableExposureLogging;
        }
    }
}
