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
        Assert.Equal("7kGqFczL8Ztc2vv3tWGmvO", experiment.RuleID);

        Assert.NotNull(experiment.EvaluationDetails);
        Assert.Equal(1730849264112, experiment.EvaluationDetails.Lcut);
        Assert.Equal("Network:Recognized", experiment.EvaluationDetails.Reason);
        Assert.Equal(1730927332613, experiment.EvaluationDetails.ReceivedAt);

        Assert.NotNull(experiment.Value);
    }
}
