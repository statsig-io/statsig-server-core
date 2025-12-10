using System.Collections.Generic;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Statsig;
using Xunit;
using WireMock.Matchers;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using WireMock.Server;
using System.Security.Cryptography;

namespace Statsig.Tests
{
    public class DataStoreTests
    {
        [Fact]
        public void DataStoreGetSetTest()
        {
            Statsig.RemoveSharedInstance();
            using var dataStore = new MockDataStore();

            CallDataStore(dataStore, "/v2/download_config_specs", "test");

            Assert.True(dataStore.InitializeCalled);
            Assert.Equal("/v2/download_config_specs", dataStore.GetCall);
            Assert.Equal("/v2/download_config_specs", dataStore.SupportPollingUpdatesForCall);
            Assert.True(dataStore.ShutdownCalled);

            var setCall = dataStore.SetCallInfo;
            Assert.NotNull(setCall);
            Assert.Equal("/v2/download_config_specs", setCall!.Key);
            Assert.Equal("test", setCall.Value);
            Assert.Equal((ulong)123, setCall.Time);
        }

        [Fact]
        public void DataStoreSetTest()
        {
            using var dataStore = new MockDataStore();
            var builder = new StatsigOptionsBuilder();

            var result = builder.SetDataStore(dataStore);

            Assert.Same(builder, result);

            var field = builder.GetType().GetField("dataStore", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            var value = field?.GetValue(builder);
            Assert.Same(dataStore, value);

            using var options = builder.Build();
            Assert.Same(dataStore, options.DataStore);
            Assert.NotEqual(0UL, dataStore.Reference);
        }

        [Fact]
        public async Task DataStorePersistsSpecsFromNetwork()
        {
            Statsig.RemoveSharedInstance();
            var specsJson = TestUtils.LoadJsonFile("eval_proj_dcs.json");

            using var server = WireMockServer.Start();
            SetupMockEndpoints(server, specsJson);

            using var dataStore = new MockDataStore
            {
                NextGetResponse = new DataStoreResponse(null, null),
                ShouldReturnPolling = true,
            };

            using var options = new StatsigOptionsBuilder()
                .SetDataStore(dataStore)
                .SetSpecsURL($"{server.Urls[0]}/v2/download_config_specs")
                .SetLogEventURL($"{server.Urls[0]}/v1/log_event")
                .SetIDListsURL($"{server.Urls[0]}/v1/get_id_lists")
                .Build();

            using var statsig = new Statsig("secret-key", options);

            await statsig.Initialize();

            Assert.True(dataStore.InitializeCalled);

            Assert.NotNull(dataStore.GetCall);
            Assert.Contains("/v2/download_config_specs", dataStore.GetCall);

            var setCall = dataStore.SetCallInfo;

            // Poll until setCallInfo is set
            int attempts = 0;
            while (attempts < 10 && setCall == null)
            {
                await Task.Delay(100);
                setCall = dataStore.SetCallInfo;
                attempts++;
            }

            Assert.True(attempts < 10, "SetCallInfo did not initialize in time");

            Assert.NotNull(setCall);
            Assert.Contains("/v2/download_config_specs", setCall!.Key);
            Assert.Equal(specsJson, setCall.Value);
            Assert.True(setCall.Time.HasValue && setCall.Time.Value > 0);

            Assert.Equal("/v2/download_config_specs", dataStore.SupportPollingUpdatesForCall);

            await statsig.Shutdown();
            Assert.True(dataStore.ShutdownCalled);
        }

        [Fact]
        public async Task DataStoreInitializesFromCachedSpecs()
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

            using var user = new StatsigUserBuilder().SetUserID("cached-user").Build();
            var config = statsig.GetDynamicConfig(user, "operating_system_config");
            Assert.Equal(13, config.Get<int>("num", 0));

            await statsig.Shutdown();

            Assert.True(dataStore.InitializeCalled);
            Assert.True(dataStore.ShutdownCalled);
            Assert.NotNull(dataStore.GetCall);
            Assert.Contains("/v2/download_config_specs", dataStore.GetCall);
            Assert.Null(dataStore.SetCallInfo);
        }

        private unsafe void CallDataStore(MockDataStore dataStore, string path, string value)
        {
            var pathBytes = StatsigUtils.ToUtf8NullTerminated(path);
            var valueBytes = StatsigUtils.ToUtf8NullTerminated(value);

            fixed (byte* pathPtr = pathBytes)
            fixed (byte* valuePtr = valueBytes)
            {
                var resultPtr = StatsigFFI.__internal__test_data_store(dataStore.Reference, pathPtr, valuePtr);
                var resultJson = StatsigUtils.ReadStringFromPointer(resultPtr);
                if (!string.IsNullOrEmpty(resultJson))
                {
                    JsonConvert.DeserializeObject<Dictionary<string, object>>(resultJson);
                }
            }
        }

        private static void SetupMockEndpoints(WireMockServer server, string specsJson)
        {
            server
                .Given(Request.Create()
                    .WithPath(new RegexMatcher(@"/v2/download_config_specs.*"))
                    .UsingGet())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody(specsJson));

            server
                .Given(Request.Create()
                    .WithPath("/v1/log_event")
                    .UsingPost())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody("{\"success\": true}"));

            server
                .Given(Request.Create()
                    .WithPath("/v1/get_id_lists")
                    .UsingPost())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody("{}"));
        }
    }
}
