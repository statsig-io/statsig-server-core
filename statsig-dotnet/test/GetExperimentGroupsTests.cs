using Xunit;
using WireMock.Server;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Linq;
using System;
using WireMock.Matchers;

namespace Statsig.Tests
{
    public class GetExperimentGroupsTests : IDisposable
    {
        private readonly WireMockServer _mockServer;
        private readonly string _testData;

        public GetExperimentGroupsTests()
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
        }

        private StatsigOptions CreateMockOptions()
        {
            return new StatsigOptionsBuilder()
                .SetSpecsURL($"{_mockServer.Urls[0]}/v2/download_config_specs")
                .SetLogEventURL($"{_mockServer.Urls[0]}/v1/log_event")
                .Build();
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsGroupsForKnownExperiment()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var groups = statsig.GetExperimentGroups("test_experiment_no_targeting");
            var groupsByName = groups.ToDictionary(g => g.GroupName, g => g.ReturnValue);

            // Only the experiment group rules are returned (the layerAssignment rule is excluded).
            Assert.Equal(3, groupsByName.Count);
            Assert.Equal("control", groupsByName["Control"]["value"].ToString());
            Assert.Equal("test_1", groupsByName["Test"]["value"].ToString());
            Assert.Equal("test_2", groupsByName["Test2"]["value"].ToString());
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsEmptyForUnknownExperiment()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var groups = statsig.GetExperimentGroups("nonexistent_experiment");
            Assert.Empty(groups);
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsEmptyForDynamicConfig()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var groups = statsig.GetExperimentGroups("test_max_dynamic_config_size_again");
            Assert.Empty(groups);
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsEmptyForInactiveExperiment()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var groups = statsig.GetExperimentGroups("an_experiment1");
            Assert.Empty(groups);
        }

        public void Dispose()
        {
            _mockServer?.Stop();
            _mockServer?.Dispose();
            GC.SuppressFinalize(this);
        }
    }
}
