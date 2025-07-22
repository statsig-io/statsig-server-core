using System.Collections.Concurrent;
using System.Threading.Tasks;
using Xunit;

namespace Statsig.Tests
{
    public class StatsigShutdownTests
    {
        private static readonly string[] ExpectedOrder = ["A", "B", "C"];

        [Fact]
        public async Task CorrectExecutionOrder()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);

            await statsig.Initialize();

            var state = new ConcurrentQueue<string>();
            state.Enqueue("A");

            var shutdownTask = statsig
                .Shutdown()
                .ContinueWith(_ => state.Enqueue("C"));

            state.Enqueue("B");

            await shutdownTask;

            var result = state.ToArray();
            Assert.Equal(ExpectedOrder, result);
        }
    }
}
