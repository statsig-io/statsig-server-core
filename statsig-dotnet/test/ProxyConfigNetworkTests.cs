using Xunit;
using WireMock.Server;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using WireMock.Matchers;
using System.Threading.Tasks;
using System;
using System.Linq;

namespace Statsig.Tests
{
    public class ProxyConfigNetworkTests : IDisposable
    {
        private readonly WireMockServer _mockServer;
        private readonly string _testData;

        public ProxyConfigNetworkTests()
        {
            _mockServer = WireMockServer.Start();
            _testData = TestUtils.LoadJsonFile("eval_proj_dcs.json");
            SetupMockEndpoints();
        }

        private void SetupMockEndpoints()
        {
            _mockServer
                .Given(Request.Create()
                    .WithPath(new RegexMatcher(@"/v2/download_config_specs/.*\.json"))
                    .UsingGet())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody(_testData));

            _mockServer
                .Given(Request.Create()
                    .WithPath("/v1/log_event")
                    .UsingPost())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody("{\"success\": true}"));

            _mockServer
                .Given(Request.Create()
                    .WithPath("/v1/get_id_lists")
                    .UsingPost())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody("{}"));
        }

        [Fact]
        public async Task ProxyConfig_AllowsNetworkOperations_AfterInitialization()
        {
            // Extract host and port from mock server URL
            var mockUri = new Uri(_mockServer.Urls[0]);
            var proxyConfig = new ProxyConfig(mockUri.Host, mockUri.Port);

            // Configure Statsig to use the mock server directly (without proxy in URLs)
            // The proxy config will be passed to the FFI layer but won't actually proxy
            // since we're hitting the same server. This tests that proxy config doesn't break requests.
            var options = new StatsigOptionsBuilder()
                .SetSpecsURL($"{_mockServer.Urls[0]}/v2/download_config_specs")
                .SetLogEventURL($"{_mockServer.Urls[0]}/v1/log_event")
                .SetIDListsURL($"{_mockServer.Urls[0]}/v1/get_id_lists")
                .SetProxyConfig(proxyConfig)
                .Build();

            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test-user").Build();

            await statsig.Initialize();

            // Verify operations work after initialization with proxy config
            var gateResult = statsig.CheckGate(user, "test_public");
            Assert.True(gateResult);

            statsig.LogEvent(user, "test_event", "test_value");

            var flushTask = statsig.FlushEvents();
            await flushTask;
            Assert.True(flushTask.IsCompletedSuccessfully);

            // Verify mock server received requests
            var logEntries = _mockServer.LogEntries.ToList();
            Assert.NotEmpty(logEntries);

            // Verify specific requests were made
            var specsRequests = logEntries.Where(e =>
                e.RequestMessage.Path.Contains("download_config_specs")).ToList();
            Assert.NotEmpty(specsRequests);

            // Verify proxy config was properly set
            Assert.Equal(mockUri.Host, proxyConfig.ProxyHost);
            Assert.Equal(mockUri.Port, proxyConfig.ProxyPort);
        }

        public void Dispose()
        {
            _mockServer?.Stop();
            _mockServer?.Dispose();
            GC.SuppressFinalize(this);
        }
    }
}

