using Xunit;

namespace Statsig.Tests
{
    public class ClientInitResponseOptionsTests
    {
        [Fact]
        public void ClientInitResponseOptions_DefaultConstructor_CreatesSuccessfully()
        {
            var options = new ClientInitResponseOptions();

            Assert.NotNull(options);
        }
    }
}
