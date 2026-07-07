using System.Threading.Tasks;
using Xunit;

namespace Statsig.Tests
{
    public class GetAutotuneListTests
    {
        [Fact]
        public async Task GetAutotuneList_ReturnsConfiguredAutotunes()
        {
            Statsig.RemoveSharedInstance();
            var cachedSpecs = TestUtils.LoadJsonFile("eval_proj_dcs.json");

            using var dataStore = new MockDataStore
            {
                NextGetResponse = new DataStoreResponse(cachedSpecs, 999),
                ShouldReturnPolling = true,
            };

            using var options = new StatsigOptionsBuilder()
                .SetDataStore(dataStore)
                .SetDisableNetwork(true)
                .Build();

            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var autotuneList = statsig.GetAutotuneList();

            Assert.NotNull(autotuneList);
            Assert.Contains("test_autotune", autotuneList);
            // The .NET Resources fixture defines exactly one autotune — assert no
            // extras leak into the list (order-independent).
            Assert.Single(autotuneList);

            await statsig.Shutdown();
        }
    }
}
