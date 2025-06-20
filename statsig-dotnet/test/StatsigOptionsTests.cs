using Xunit;
using System.Reflection;

namespace Statsig.Tests
{
    public class StatsigOptionsTests
    {
        [Fact]
        public void StatsigOptionsBuilder_SetSpecsURL_SetsSpecsURLCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testUrl = "https://api.statsig.com/v1/download_config_specs";

            var result = builder.SetSpecsURL(testUrl);

            Assert.Same(builder, result);
            Assert.Equal(testUrl, GetInternalField(builder, "specsURL"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetLogEventURL_SetsLogEventURLCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testUrl = "https://api.statsig.com/v1/log_event";

            var result = builder.SetLogEventURL(testUrl);

            Assert.Same(builder, result);
            Assert.Equal(testUrl, GetInternalField(builder, "logEventURL"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetEnvironment_SetsEnvironmentCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testEnvironment = "staging";

            var result = builder.SetEnvironment(testEnvironment);

            Assert.Same(builder, result);
            Assert.Equal(testEnvironment, GetInternalField(builder, "environment"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetSpecsSyncIntervalMs_SetsIntervalCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testInterval = 30000;

            var result = builder.SetSpecsSyncIntervalMs(testInterval);

            Assert.Same(builder, result);
            Assert.Equal(testInterval, GetInternalField(builder, "specsSyncIntervalMs"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetWaitForCountryLookupInit_SetsValueCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testValue = true;

            var result = builder.SetWaitForCountryLookupInit(testValue);

            Assert.Same(builder, result);
            Assert.Equal(testValue, GetInternalField(builder, "waitForCountryLookupInit"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetWaitForUserAgentInit_SetsValueCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testValue = true;

            var result = builder.SetWaitForUserAgentInit(testValue);

            Assert.Same(builder, result);
            Assert.Equal(testValue, GetInternalField(builder, "waitForUserAgentInit"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetEnableIDLists_SetsValueCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testValue = true;

            var result = builder.SetEnableIDLists(testValue);

            Assert.Same(builder, result);
            Assert.Equal(testValue, GetInternalField(builder, "enableIDLists"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetIDListsURL_SetsURLCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testUrl = "https://api.statsig.com/v1/get_id_lists";

            var result = builder.SetIDListsURL(testUrl);

            Assert.Same(builder, result);
            Assert.Equal(testUrl, GetInternalField(builder, "idListsURL"));
        }

        [Fact]
        public void StatsigOptionsBuilder_SetIDListsSyncIntervalMs_SetsIntervalCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var testInterval = 60000;

            var result = builder.SetIDListsSyncIntervalMs(testInterval);

            Assert.Same(builder, result);
            Assert.Equal(testInterval, GetInternalField(builder, "idListsSyncIntervalMs"));
        }

        [Fact]
        public void StatsigOptionsBuilder_Build_CreatesStatsigOptionsSuccessfully()
        {
            var builder = new StatsigOptionsBuilder();

            using var options = builder.Build();

            Assert.NotNull(options);
        }

        [Fact]
        public void StatsigOptionsBuilder_Build_WithAllProperties_CreatesStatsigOptionsSuccessfully()
        {
            var builder = new StatsigOptionsBuilder()
                .SetSpecsURL("https://custom.statsig.com/specs")
                .SetLogEventURL("https://custom.statsig.com/events")
                .SetEnvironment("production")
                .SetSpecsSyncIntervalMs(45000)
                .SetWaitForCountryLookupInit(true)
                .SetWaitForUserAgentInit(false)
                .SetEnableIDLists(true)
                .SetIDListsURL("https://custom.statsig.com/id_lists")
                .SetIDListsSyncIntervalMs(90000);

            using var options = builder.Build();

            Assert.NotNull(options);
        }

        [Fact]
        public void StatsigOptionsBuilder_AllSetterMethods_SetInternalFieldsCorrectly()
        {
            var builder = new StatsigOptionsBuilder()
                .SetSpecsURL("https://custom.statsig.com/specs")
                .SetLogEventURL("https://custom.statsig.com/events")
                .SetEnvironment("production")
                .SetSpecsSyncIntervalMs(45000)
                .SetWaitForCountryLookupInit(true)
                .SetWaitForUserAgentInit(false)
                .SetEnableIDLists(true)
                .SetIDListsURL("https://custom.statsig.com/id_lists")
                .SetIDListsSyncIntervalMs(90000);

            Assert.Equal("https://custom.statsig.com/specs", GetInternalField(builder, "specsURL"));
            Assert.Equal("https://custom.statsig.com/events", GetInternalField(builder, "logEventURL"));
            Assert.Equal("production", GetInternalField(builder, "environment"));
            Assert.Equal(45000, GetInternalField(builder, "specsSyncIntervalMs"));
            Assert.Equal(true, GetInternalField(builder, "waitForCountryLookupInit"));
            Assert.Equal(false, GetInternalField(builder, "waitForUserAgentInit"));
            Assert.Equal(true, GetInternalField(builder, "enableIDLists"));
            Assert.Equal("https://custom.statsig.com/id_lists", GetInternalField(builder, "idListsURL"));
            Assert.Equal(90000, GetInternalField(builder, "idListsSyncIntervalMs"));
        }

        [Fact]
        public void StatsigOptions_Dispose_DoesNotThrow()
        {
            var builder = new StatsigOptionsBuilder();
            var options = builder.Build();

            options.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void StatsigOptions_DisposeMultipleTimes_DoesNotThrow()
        {
            var builder = new StatsigOptionsBuilder();
            var options = builder.Build();

            options.Dispose();
            options.Dispose();

            Assert.True(true);
        }

        [Fact]
        public void StatsigOptionsBuilder_CanBeCreated_Successfully()
        {
            var builder = new StatsigOptionsBuilder();

            Assert.NotNull(builder);
        }

        private static object? GetInternalField(object obj, string fieldName)
        {
            var field = obj.GetType().GetField(fieldName, BindingFlags.NonPublic | BindingFlags.Instance);
            return field?.GetValue(obj);
        }
    }
}
