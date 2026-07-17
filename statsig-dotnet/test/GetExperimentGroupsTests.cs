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

            var result = statsig.GetExperimentGroups("test_experiment_no_targeting");
            Assert.True(result.IsExperimentActive);

            var groupsByName = result.Groups.ToDictionary(g => g.GroupName, g => g);

            // Only the experiment group rules are returned (the layerAssignment rule is excluded).
            Assert.Equal(3, groupsByName.Count);
            Assert.Equal("control", groupsByName["Control"].ReturnValue["value"].ToString());
            Assert.Equal("54QJztEPRLXK7ZCvXeY9q4", groupsByName["Control"].RuleID);
            Assert.Equal("userID", groupsByName["Control"].IDType);
            Assert.Equal("test_1", groupsByName["Test"].ReturnValue["value"].ToString());
            Assert.Equal("test_2", groupsByName["Test2"].ReturnValue["value"].ToString());
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsNullActiveStateForUnknownExperiment()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var result = statsig.GetExperimentGroups("nonexistent_experiment");
            Assert.Null(result.IsExperimentActive);
            Assert.Empty(result.Groups);
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsNullActiveStateForDynamicConfig()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            // operating_system_config is a dynamic config in the dotnet copy of eval_proj_dcs.json.
            var result = statsig.GetExperimentGroups("operating_system_config");
            Assert.Null(result.IsExperimentActive);
            Assert.Empty(result.Groups);
        }

        [Fact]
        public async Task GetExperimentGroups_ReturnsGroupsForInactiveExperiment()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            // test_switchback has isActive: false; groups are still returned along with the flag.
            var result = statsig.GetExperimentGroups("test_switchback");
            Assert.False(result.IsExperimentActive);

            // Only the experiment group rules are returned (non-group rules are excluded).
            var groupNames = result.Groups.Select(g => g.GroupName).OrderBy(n => n).ToList();
            Assert.Equal(["Control", "Test"], groupNames);
        }

        public void Dispose()
        {
            _mockServer?.Stop();
            _mockServer?.Dispose();
            GC.SuppressFinalize(this);
        }
    }
}
