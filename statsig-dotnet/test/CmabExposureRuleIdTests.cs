using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using WireMock.Matchers;
using WireMock.RequestBuilders;
using WireMock.ResponseBuilders;
using WireMock.Server;
using Xunit;

namespace Statsig.Tests
{
    /// <summary>
    /// .NET uses the typed experiment path (statsig_get_experiment -> get_experiment).
    /// </summary>
    public class CmabExposureRuleIdTests : IAsyncLifetime
    {
        private readonly WireMockServer _mockServer;
        private Statsig? _statsig;

        public CmabExposureRuleIdTests()
        {
            _mockServer = WireMockServer.Start();

            _mockServer
                .Given(Request.Create()
                    .WithPath(new RegexMatcher(@"/v2/download_config_specs/.*\.json"))
                    .UsingGet())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody(TestUtils.LoadJsonFile("dcs_cmab_exposure_rule_id.json")));

            _mockServer
                .Given(Request.Create().WithPath("/v1/log_event").UsingPost())
                .RespondWith(Response.Create()
                    .WithStatusCode(200)
                    .WithHeader("Content-Type", "application/json")
                    .WithBody("{\"success\": true}"));
        }

        public async Task InitializeAsync()
        {
            var options = new StatsigOptionsBuilder()
                .SetSpecsURL($"{_mockServer.Urls[0]}/v2/download_config_specs")
                .SetLogEventURL($"{_mockServer.Urls[0]}/v1/log_event")
                .Build();

            _statsig = new Statsig("secret-key", options);
            await _statsig.Initialize();
        }

        public async Task DisposeAsync()
        {
            if (_statsig != null)
            {
                await _statsig.Shutdown();
            }
            _mockServer.Stop();
            _mockServer.Dispose();
        }

        [Fact]
        public async Task ExploreSuffixPreservedInAutomaticExposure()
        {
            var user = new StatsigUserBuilder().SetUserID("typed-explore-user").Build();

            var experiment = _statsig!.GetExperiment(user, "cmab_always_explore");
            await _statsig.FlushEvents();

            Assert.EndsWith(":explore", experiment.RuleID);
            Assert.Equal(experiment.RuleID, GetLoggedRuleId());
        }

        [Fact]
        public async Task ExploitRuleIdRemainsBareInAutomaticExposure()
        {
            var user = new StatsigUserBuilder().SetUserID("typed-exploit-user").Build();

            var experiment = _statsig!.GetExperiment(user, "cmab_never_explore");
            await _statsig.FlushEvents();

            Assert.Equal("treatment-group-id", experiment.RuleID);
            Assert.Equal(experiment.RuleID, GetLoggedRuleId());
        }

        private string GetLoggedRuleId()
        {
            var ruleIds = _mockServer.LogEntries
                .Where(e => e.RequestMessage.Path == "/v1/log_event")
                .SelectMany(e => ReadEvents(e.RequestMessage))
                .Where(e => (string)e["eventName"] == "statsig::config_exposure")
                .Select(e => (string)e["metadata"]?["ruleID"])
                .ToList();

            Assert.Single(ruleIds);
            return ruleIds[0];
        }

        private static IEnumerable<JToken> ReadEvents(WireMock.IRequestMessage request)
        {
            string json;
            if (request.BodyAsJson != null)
            {
                json = JsonConvert.SerializeObject(request.BodyAsJson);
            }
            else
            {
                var bytes = request.BodyAsBytes ?? Encoding.UTF8.GetBytes(request.Body ?? "");
                json = IsGzip(bytes) ? Gunzip(bytes) : Encoding.UTF8.GetString(bytes);
            }

            return JObject.Parse(json)["events"]?.Children() ?? Enumerable.Empty<JToken>();
        }

        private static bool IsGzip(byte[] bytes)
        {
            return bytes.Length > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b;
        }

        private static string Gunzip(byte[] bytes)
        {
            using var input = new GZipStream(new MemoryStream(bytes), CompressionMode.Decompress);
            using var reader = new StreamReader(input, Encoding.UTF8);
            return reader.ReadToEnd();
        }
    }
}
