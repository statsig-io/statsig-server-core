using Newtonsoft.Json;

namespace Statsig
{
    public class EvaluationDetails
    {
        [JsonProperty("lcut")] public long Lcut = 0;
        [JsonProperty("received_at")] public long ReceivedAt = 0;
        [JsonProperty("reason")] public string Reason = "";

        internal EvaluationDetails(long lcut, long receivedAt, string reason)
        {
            Lcut = lcut;
            ReceivedAt = receivedAt;
            Reason = reason;
        }
    }
}