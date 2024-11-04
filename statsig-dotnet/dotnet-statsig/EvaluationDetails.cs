namespace StatsigServer
{
    public class EvaluationDetails
    {
        public long Lcut;
        public long ReceivedAt;
        public string Reason;

        internal EvaluationDetails(long lcut, long receivedAt, string reason)
        {
            Lcut = lcut;
            ReceivedAt = receivedAt;
            Reason = reason;
        }
    }
}