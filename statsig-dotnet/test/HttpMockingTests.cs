using Xunit;
using WireMock.Server;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using System.Threading.Tasks;
using System.Collections.Generic;
using System;
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
        public async Task InitializeCallback_WhenCalled_InvokesCallbackSuccessfully()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);

            var initializeTask = statsig.Initialize();

            await initializeTask;

            Assert.True(initializeTask.IsCompletedSuccessfully);
            Assert.False(initializeTask.IsFaulted);
            Assert.False(initializeTask.IsCanceled);
        }

        [Fact]
        public async Task InitializeCallback_WithMultipleCalls_HandlesGracefully()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);

            var task1 = statsig.Initialize();
            var task2 = statsig.Initialize();
            var task3 = statsig.Initialize();

            await Task.WhenAll(task1, task2, task3);

            Assert.True(task1.IsCompletedSuccessfully);
            Assert.True(task2.IsCompletedSuccessfully);
            Assert.True(task3.IsCompletedSuccessfully);
        }

        [Fact]
        public async Task InitializeCallback_WithConcurrentCalls_CompletesCorrectly()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);

            var tasks = new List<Task>();
            for (int i = 0; i < 5; i++)
            {
                tasks.Add(statsig.Initialize());
            }

            await Task.WhenAll(tasks);

            foreach (var task in tasks)
            {
                Assert.True(task.IsCompletedSuccessfully);
                Assert.False(task.IsFaulted);
            }
        }

        [Fact]
        public async Task InitializeCallback_TimingVerification_CompletesWithinReasonableTime()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);

            var stopwatch = System.Diagnostics.Stopwatch.StartNew();
            await statsig.Initialize();
            stopwatch.Stop();

            Assert.True(stopwatch.ElapsedMilliseconds < 5000,
                $"Initialize took too long: {stopwatch.ElapsedMilliseconds}ms");
        }

        [Fact]
        public async Task InitializeCallback_AfterInitialization_AllowsSubsequentOperations()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test-user").Build();

            await statsig.Initialize();

            var gateResult = statsig.CheckGate(user, "test_gate");
            statsig.LogEvent(user, "test_event", "test_value");

            var flushTask = statsig.FlushEvents();
            await flushTask;
            Assert.True(flushTask.IsCompletedSuccessfully);
        }

        [Fact]
        public async Task GetExperiment_NormalPath_GroupNameSemantics()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test-user").Build();
            await statsig.Initialize();

            // Recognized experiment: the normal (snake_case) path surfaces the
            // assigned group name as a non-null, non-empty string.
            var experiment = statsig.GetExperiment(user, "test_experiment_no_targeting");
            Assert.True(
                experiment.GroupName is "Control" or "Test",
                $"Unexpected group name: {experiment.GroupName}");

            // Unrecognized experiment: group_name arrives as JSON null on the
            // normal path, which must map to C# null (not ""). Locks the
            // JTokenType.Null handling in Experiment.cs against a regression.
            var unknown = statsig.GetExperiment(user, "not_a_real_experiment");
            Assert.Null(unknown.GroupName);
        }

        [Fact]
        public async Task GetExperimentByGroupName_ReturnsMatchingGroup()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            await statsig.Initialize();

            var control = statsig.GetExperimentByGroupName("test_experiment_no_targeting", "Control");
            Assert.Equal("Control", control.GroupName);
            Assert.Equal("54QJztEPRLXK7ZCvXeY9q4", control.RuleID);
            Assert.Equal("userID", control.IDType);
            Assert.Equal("control", control.Value["value"].ToString());

            var test = statsig.GetExperimentByGroupName("test_experiment_no_targeting", "Test");
            Assert.Equal("Test", test.GroupName);
            Assert.Equal("test_1", test.Value["value"].ToString());
        }

        [Fact]
        public async Task GetExperimentByGroupName_Unrecognized_ReturnsEmpty()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            await statsig.Initialize();

            var experiment = statsig.GetExperimentByGroupName("not_an_experiment", "Control");
            Assert.Null(experiment.GroupName);
            Assert.Equal(string.Empty, experiment.RuleID);
        }

        [Fact]
        public async Task GetExperimentByGroupName_UnknownGroup_ReturnsEmpty()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            await statsig.Initialize();

            var experiment = statsig.GetExperimentByGroupName("test_experiment_no_targeting", "NotAGroup");
            Assert.Null(experiment.GroupName);
            Assert.Equal(string.Empty, experiment.RuleID);
        }

        [Fact]
        public async Task GetExperimentByGroupIdAdvanced_ReturnsMatchingGroup()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            await statsig.Initialize();

            var experiment = statsig.GetExperimentByGroupIdAdvanced(
                "test_experiment_no_targeting", "54QJztEPRLXK7ZCvXeY9q4");
            Assert.Equal("Control", experiment.GroupName);
            Assert.Equal("control", experiment.Value["value"].ToString());
        }

        [Fact]
        public async Task GetExperimentByGroupIdAdvanced_Unrecognized_ReturnsEmpty()
        {
            var options = CreateMockOptions();
            using var statsig = new Statsig("secret-test-key", options);
            await statsig.Initialize();

            var experiment = statsig.GetExperimentByGroupIdAdvanced(
                "test_experiment_no_targeting", "not_a_group_id");
            Assert.Null(experiment.GroupName);
            Assert.Equal(string.Empty, experiment.RuleID);
        }

        public void Dispose()
        {
            _mockServer?.Stop();
            _mockServer?.Dispose();
            GC.SuppressFinalize(this);
        }
    }
}
