using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Xunit;

namespace Statsig.Tests
{
    public class OverrideParamStoreTests
    {
        private static readonly int[] value = [1, 2, 3, 4, 5];

        [Fact]
        public async Task TestOverrideParamStore()
        {
            Statsig.RemoveSharedInstance();
            var _specsJson = TestUtils.LoadJsonFile("param_store_override_specs.json");

            using var dataStore = new MockDataStore
            {
                NextGetResponse = new DataStoreResponse(_specsJson, 1),
                ShouldReturnPolling = true,
            };

            using var options = new StatsigOptionsBuilder()
                .SetDataStore(dataStore)
                .SetDisableNetwork(true)
                .Build();

            using var statsig = new Statsig("secret-key", options);
            await statsig.Initialize();

            var user = new StatsigUserBuilder().SetUserID("user_a").Build();
            var paramStore = statsig.GetParameterStore(user, "testing123");

            // Verify initial values from specs
            Assert.Equal(
                "Testing_my_string_value",
                paramStore.GetString("TestString", "default value")
            );

            statsig.OverrideParameterStore("testing123", new Dictionary<string, object>()
            {
                { "TestString", "overridden value" },
                { "bool_param", true },
                { "int_param", 42 },
                { "double_param", 3.14 },
                { "array_param", value },
                { "object_param", new Dictionary<string, object>()
                    {
                        { "nested_key", "nested_value" }
                    }
                }
            });

            var overriddenParamStore = statsig.GetParameterStore(user, "testing123");

            Assert.Equal(
                "overridden value",
                overriddenParamStore.GetString("TestString", "didn't work")
            );

            Assert.True(
                overriddenParamStore.GetBool("bool_param", false)
            );

            Console.WriteLine(overriddenParamStore.ToString());

            Assert.Equal(42, overriddenParamStore.GetLong("int_param", 0));

            Assert.Equal(3.14, overriddenParamStore.GetDouble("double_param", 0.0));

            var arrayParam = overriddenParamStore.GetList("array_param", ["d", "e", "f"]);
            Assert.Equal(5, arrayParam.Count);
            // Equivalent used to avoid implicit casting issues
            Assert.Equivalent("1", arrayParam[0]);
            Assert.Equivalent("2", arrayParam[1]);
            Assert.Equivalent("3", arrayParam[2]);

            var objectParam = overriddenParamStore.GetDictionary("object_param", []);
            Assert.True(objectParam.ContainsKey("nested_key"));
            Assert.Equal("nested_value", objectParam["nested_key"].ToString());

            await statsig.Shutdown();
        }
    }
}
