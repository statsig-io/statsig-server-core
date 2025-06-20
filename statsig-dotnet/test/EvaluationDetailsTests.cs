using Xunit;

namespace Statsig.Tests
{
    public class EvaluationDetailsTests
    {
        [Fact]
        public void EvaluationDetails_Constructor_SetsPropertiesCorrectly()
        {
            var lcut = 1234567890L;
            var receivedAt = 9876543210L;
            var reason = "Network:Recognized";

            var details = new EvaluationDetails(lcut, receivedAt, reason);

            Assert.Equal(lcut, details.Lcut);
            Assert.Equal(receivedAt, details.ReceivedAt);
            Assert.Equal(reason, details.Reason);
        }

        [Fact]
        public void EvaluationDetails_Constructor_WithEmptyReason_SetsCorrectly()
        {
            var details = new EvaluationDetails(123L, 456L, string.Empty);

            Assert.Equal(123L, details.Lcut);
            Assert.Equal(456L, details.ReceivedAt);
            Assert.Equal(string.Empty, details.Reason);
        }

        [Fact]
        public void EvaluationDetails_Constructor_WithZeroValues_SetsCorrectly()
        {
            var details = new EvaluationDetails(0L, 0L, "LocalOverride");

            Assert.Equal(0L, details.Lcut);
            Assert.Equal(0L, details.ReceivedAt);
            Assert.Equal("LocalOverride", details.Reason);
        }

        [Fact]
        public void EvaluationDetails_Constructor_WithNegativeValues_SetsCorrectly()
        {
            var details = new EvaluationDetails(-1L, -1L, "Error");

            Assert.Equal(-1L, details.Lcut);
            Assert.Equal(-1L, details.ReceivedAt);
            Assert.Equal("Error", details.Reason);
        }

        [Fact]
        public void EvaluationDetails_Constructor_WithNullValues_SetsCorrectly()
        {
            var details = new EvaluationDetails(null, null, "Network:Recognized");

            Assert.Null(details.Lcut);
            Assert.Null(details.ReceivedAt);
            Assert.Equal("Network:Recognized", details.Reason);
        }

        [Fact]
        public void EvaluationDetails_DefaultValues_AreSetCorrectly()
        {
            var nullDetails = new EvaluationDetails(null, null, "");
            Assert.Null(nullDetails.Lcut);
            Assert.Null(nullDetails.ReceivedAt);
            Assert.Equal("", nullDetails.Reason);

            var nonNullDetails = new EvaluationDetails(0L, 0L, "");
            Assert.Equal(0L, nonNullDetails.Lcut);
            Assert.Equal(0L, nonNullDetails.ReceivedAt);
            Assert.Equal("", nonNullDetails.Reason);
        }
    }
}
