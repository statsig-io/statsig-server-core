using Xunit;

namespace Statsig.Tests
{
    public class EvaluationOptionsTests
    {
        [Fact]
        public void EvaluationOptions_DefaultConstructor_SetsDefaultValues()
        {
            var options = new EvaluationOptions();

            Assert.False(options.DisableExposureLogging);
        }

        [Fact]
        public void EvaluationOptions_Constructor_WithTrue_SetsCorrectly()
        {
            var options = new EvaluationOptions(disableExposureLogging: true);

            Assert.True(options.DisableExposureLogging);
        }

        [Fact]
        public void EvaluationOptions_Constructor_WithFalse_SetsCorrectly()
        {
            var options = new EvaluationOptions(disableExposureLogging: false);

            Assert.False(options.DisableExposureLogging);
        }

        [Fact]
        public void EvaluationOptions_Property_CanBeSet()
        {
            var options = new EvaluationOptions
            {
                DisableExposureLogging = true
            };

            Assert.True(options.DisableExposureLogging);
        }

        [Fact]
        public void EvaluationOptions_Property_CanBeUnset()
        {
            var options = new EvaluationOptions(disableExposureLogging: true)
            {
                DisableExposureLogging = false
            };

            Assert.False(options.DisableExposureLogging);
        }
    }
}
