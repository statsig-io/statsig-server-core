using Xunit;
using Moq;
using System.Threading.Tasks;
using System.Collections.Generic;
using System;
using System.IO;

namespace Statsig.Tests
{
    public class StatsigTests
    {
        [Fact]
        public void Statsig_Constructor_WithValidParameters_CreatesSuccessfully()
        {
            var options = new StatsigOptionsBuilder().Build();

            using var statsig = new Statsig("secret-test-key", options);

            Assert.NotNull(statsig);
        }

        [Fact]
        public void Statsig_Constructor_WithNullKey_ThrowsException()
        {
            var options = new StatsigOptionsBuilder().Build();

            Assert.Throws<System.ArgumentNullException>(() => new Statsig(null, options));
        }

        [Fact]
        public void Statsig_Constructor_WithEmptyKey_CreatesSuccessfully()
        {
            var options = new StatsigOptionsBuilder().Build();

            using var statsig = new Statsig("", options);

            Assert.NotNull(statsig);
        }

        [Fact]
        public async Task Statsig_Initialize_CompletesSuccessfully()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);

            await statsig.Initialize();

            Assert.True(true);
        }

        [Fact]
        public void Statsig_CheckGate_WithValidParameters_ReturnsBoolean()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.CheckGate(user, "test_gate");

            Assert.IsType<bool>(result);
        }

        [Fact]
        public void Statsig_CheckGate_WithEvaluationOptions_ReturnsBoolean()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var evalOptions = new EvaluationOptions(disableExposureLogging: true);

            var result = statsig.CheckGate(user, "test_gate", evalOptions);

            Assert.IsType<bool>(result);
        }

        [Fact]
        public void Statsig_GetFeatureGate_WithValidParameters_ReturnsFeatureGate()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.GetFeatureGate(user, "test_gate");

            Assert.NotNull(result);
            Assert.IsType<FeatureGate>(result);
        }

        [Fact]
        public void Statsig_GetFeatureGate_WithEvaluationOptions_ReturnsFeatureGate()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var evalOptions = new EvaluationOptions(disableExposureLogging: true);

            var result = statsig.GetFeatureGate(user, "test_gate", evalOptions);

            Assert.NotNull(result);
            Assert.IsType<FeatureGate>(result);
        }

        [Fact]
        public void Statsig_ManuallyLogGateExposure_WithValidParameters_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.ManuallyLogGateExposure(user, "test_gate");

            Assert.True(true);
        }

        [Fact]
        public void Statsig_GetConfig_WithValidParameters_ReturnsDynamicConfig()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.GetDynamicConfig(user, "test_config");

            Assert.NotNull(result);
            Assert.IsType<DynamicConfig>(result);
        }

        [Fact]
        public void Statsig_GetConfig_WithEvaluationOptions_ReturnsDynamicConfig()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var evalOptions = new EvaluationOptions(disableExposureLogging: true);

            var result = statsig.GetDynamicConfig(user, "test_config", evalOptions);

            Assert.NotNull(result);
            Assert.IsType<DynamicConfig>(result);
        }

        [Fact]
        public void Statsig_ManuallyLogDynamicConfigExposure_WithValidParameters_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.ManuallyLogDynamicConfigExposure(user, "test_config");

            Assert.True(true);
        }

        [Fact]
        public void Statsig_GetExperiment_WithValidParameters_ReturnsExperiment()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.GetExperiment(user, "test_experiment");

            Assert.NotNull(result);
            Assert.IsType<Experiment>(result);
        }

        [Fact]
        public void Statsig_GetExperiment_WithEvaluationOptions_ReturnsExperiment()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var evalOptions = new EvaluationOptions(disableExposureLogging: true);

            var result = statsig.GetExperiment(user, "test_experiment", evalOptions);

            Assert.NotNull(result);
            Assert.IsType<Experiment>(result);
        }

        [Fact]
        public void Statsig_ManuallyLogExperimentExposure_WithValidParameters_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.ManuallyLogExperimentExposure(user, "test_experiment");

            Assert.True(true);
        }

        [Fact]
        public void Statsig_GetLayer_WithValidParameters_ReturnsLayer()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.GetLayer(user, "test_layer");

            Assert.NotNull(result);
            Assert.IsType<Layer>(result);
        }

        [Fact]
        public void Statsig_GetLayer_WithEvaluationOptions_ReturnsLayer()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var evalOptions = new EvaluationOptions(disableExposureLogging: true);

            var result = statsig.GetLayer(user, "test_layer", evalOptions);

            Assert.NotNull(result);
            Assert.IsType<Layer>(result);
        }

        [Fact]
        public void Statsig_ManuallyLogLayerParameterExposure_WithValidParameters_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.ManuallyLogLayerParameterExposure(user, "test_layer", "test_param");

            Assert.True(true);
        }

        [Fact]
        public void Statsig_GetClientInitializeResponse_WithValidParameters_ReturnsString()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            var result = statsig.GetClientInitializeResponse(user);

            Assert.NotNull(result);
            Assert.IsType<string>(result);
        }

        [Fact]
        public void Statsig_LogEvent_WithStringValue_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.LogEvent(user, "test_event", "string_value");

            Assert.True(true);
        }

        [Fact]
        public void Statsig_LogEvent_WithIntValue_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.LogEvent(user, "test_event", 42);

            Assert.True(true);
        }

        [Fact]
        public void Statsig_LogEvent_WithDoubleValue_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.LogEvent(user, "test_event", 3.14);

            Assert.True(true);
        }

        [Fact]
        public void Statsig_LogEvent_WithMetadata_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();
            var metadata = new Dictionary<string, string> { { "key", "value" } };

            statsig.LogEvent(user, "test_event", "value", metadata);

            Assert.True(true);
        }

        [Fact]
        public void Statsig_Identify_WithValidUser_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);
            using var user = new StatsigUserBuilder().SetUserID("test_user").Build();

            statsig.Identify(user);

            Assert.True(true);
        }

        [Fact]
        public async Task Statsig_FlushEvents_CompletesSuccessfully()
        {
            var options = new StatsigOptionsBuilder().Build();
            using var statsig = new Statsig("secret-test-key", options);

            await statsig.FlushEvents();

            Assert.True(true);
        }

        [Fact]
        public void Statsig_Shutdown_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            var statsig = new Statsig("secret-test-key", options);

            statsig.Shutdown();

            Assert.True(true);
        }

        [Fact]
        public void Statsig_Dispose_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            var statsig = new Statsig("secret-test-key", options);

            statsig.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void Statsig_DisposeMultipleTimes_DoesNotThrow()
        {
            var options = new StatsigOptionsBuilder().Build();
            var statsig = new Statsig("secret-test-key", options);

            statsig.Dispose();
            statsig.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void Statsig_SharedInstance_Returns_Error_Instance_When_Already_Exists()
        {
            Statsig.RemoveSharedInstance();
            var options = new StatsigOptionsBuilder().Build();

            Assert.NotNull(Statsig.NewShared("secret-singleton", options));

            var errorInstance = Statsig.NewShared("secret-test", options);
            Assert.NotNull(errorInstance);
            Assert.NotSame(Statsig.NewShared("secret-singleton", options), errorInstance);
        }

        [Fact]
        public void Statsig_SharedInstance_Accessible_After_Creation()
        {
            Statsig.RemoveSharedInstance();
            var options = new StatsigOptionsBuilder().Build();
            var instance = Statsig.NewShared("singleton-access", options);
            Assert.NotNull(instance);
            var shared = Statsig.Shared();
            Assert.Same(instance, shared);
        }
    }
}
