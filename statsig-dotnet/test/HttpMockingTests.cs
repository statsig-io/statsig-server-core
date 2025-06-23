using Xunit;
using WireMock.Server;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using System.Threading.Tasks;
using System.Collections.Generic;
using System;
using System.Linq;
using WireMock.Matchers;

namespace Statsig.Tests
{
    public class HttpMockingTests : IDisposable
    {
        private readonly WireMockServer _mockServer;
        private readonly string _testData;

        public HttpMockingTests()
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

        private StatsigOptions CreateMockOptions()
        {
            return new StatsigOptionsBuilder()
                .SetSpecsURL($"{_mockServer.Urls[0]}/v2/download_config_specs")
                .SetLogEventURL($"{_mockServer.Urls[0]}/v1/log_event")
                .SetIDListsURL($"{_mockServer.Urls[0]}/v1/get_id_lists")
                .Build();
        }

        [Fact]
        public async Task HttpMocking_VerifyRequestsAreMade_ChecksEndpointCalls()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("a-user").Build();

            await statsig.Initialize();
            statsig.LogEvent(user, "test_event", "test_value");
            await statsig.FlushEvents();

            var configSpecsRequests = _mockServer.LogEntries.Where(x => x.RequestMessage.Path.Contains("download_config_specs"));
            var logEventRequests = _mockServer.LogEntries.Where(x => x.RequestMessage.Path.Contains("log_event"));

            Assert.True(configSpecsRequests.Any());
            Assert.True(logEventRequests.Any());
        }

        public void Dispose()
        {
            _mockServer?.Stop();
            _mockServer?.Dispose();
            GC.SuppressFinalize(this);
        }
    }
}
