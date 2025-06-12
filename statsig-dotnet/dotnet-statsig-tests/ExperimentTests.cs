using Newtonsoft.Json;
using Xunit;

namespace Statsig.Tests;

public class ExperimentTests
{
    [Fact]
    public void TestExperimentDeserialization()
    {
        var jsonString = TestUtils.LoadJsonFile("Experiment.json");
        var experiment = JsonConvert.DeserializeObject<Experiment>(jsonString);

        Assert.NotNull(experiment);
        Assert.Equal("experiment_with_many_params", experiment.Name);
        Assert.Equal("7kGqFczL8Ztc2vv3tWGmvO", experiment.RuleId);

        Assert.NotNull(experiment.Details);
        Assert.Equal(1730849264112, experiment.Details.Lcut);
        Assert.Equal("Network:Recognized", experiment.Details.Reason);
        Assert.Equal(1730927332613, experiment.Details.ReceivedAt);

        Assert.NotNull(experiment.Value);
    }
}
