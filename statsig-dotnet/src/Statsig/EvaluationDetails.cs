using Newtonsoft.Json;

namespace Statsig
{
    public class EvaluationDetails
    {
        [JsonProperty("lcut")] public long? Lcut = null;
        [JsonProperty("received_at")] public long? ReceivedAt = null;
        [JsonProperty("reason")] public string Reason = "";

        public EvaluationDetails(long? lcut, long? receivedAt, string reason)
        {
            Lcut = lcut;
            ReceivedAt = receivedAt;
            Reason = reason;
        }
    }
}
