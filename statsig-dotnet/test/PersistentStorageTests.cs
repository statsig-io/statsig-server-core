
using System.Collections.Generic;
using Newtonsoft.Json;
using Xunit;

namespace Statsig.Tests
{
    public class PersistentStorageTests
    {
        [Fact]
        public void TestPersistentStorageCreate()
        {
            Statsig.RemoveSharedInstance();
            using var storage = new MockPersistentStorage();

            Assert.NotEqual(0UL, storage.Reference);
        }

        [Fact]
        public unsafe void TestPersistentStorageLoad()
        {
            Statsig.RemoveSharedInstance();
            using var storage = new MockPersistentStorage();

            CallPersistentStorage(storage, "load", "user_123:userID");

            Assert.Single(storage.LoadCalls);
            Assert.Equal("user_123:userID", storage.LoadCalls[0]);
        }

        [Fact]
        public unsafe void TestPersistentStorageSave()
        {
            Statsig.RemoveSharedInstance();
            using var storage = new MockPersistentStorage();

            var stickyValues = new StickyValues
            {
                Value = true,
                JsonValue = null,
                RuleId = "test_rule",
                GroupName = "test_group",
                SecondaryExposures = [],
                UndelegatedSecondaryExposures = [],
                ConfigDelegate = null,
                ExplicitParameters = [],
                Time = 1234567890,
                ConfigVersion = 1
            };

            CallPersistentStorage(
                storage,
                "save",
                "user_123:userID",
                "test_experiment",
                JsonConvert.SerializeObject(stickyValues));

            Assert.Single(storage.SaveCalls);

            var call = storage.SaveCalls[0];
            Assert.Equal("user_123:userID", call.Key);
            Assert.Equal("test_experiment", call.ConfigName);
            Assert.IsType<StickyValues>(call.Data);
            Assert.True(call.Data.Value);
            Assert.Equal("test_rule", call.Data.RuleId);
        }

        [Fact]
        public unsafe void TestPersistentStorageDelete()
        {
            Statsig.RemoveSharedInstance();
            using var storage = new MockPersistentStorage();

            CallPersistentStorage(storage, "delete", "user_123:userID", "test_experiment");

            Assert.Single(storage.DeleteCalls);
            var call = storage.DeleteCalls[0];
            Assert.Equal("user_123:userID", call.Key);
            Assert.Equal("test_experiment", call.ConfigName);
        }

        [Fact]
        public void TestPersistentStorageWithStatsigOptions()
        {
            Statsig.RemoveSharedInstance();
            using var storage = new MockPersistentStorage();

            using var options = new StatsigOptionsBuilder()
                .SetPersistentStorage(storage)
                .Build();

            Assert.NotNull(options);
            Assert.Same(storage, options.PersistentStorage);
            Assert.NotEqual(0UL, options.Reference);
        }

        private unsafe void CallPersistentStorage(
            MockPersistentStorage storage,
            string action,
            string key,
            string configName = "",
            string data = "")
        {
            var actionBytes = StatsigUtils.ToUtf8NullTerminated(action);
            var keyBytes = StatsigUtils.ToUtf8NullTerminated(key);
            var configBytes = StatsigUtils.ToUtf8NullTerminated(configName ?? string.Empty);
            var dataBytes = StatsigUtils.ToUtf8NullTerminated(data ?? string.Empty);

            fixed (byte* actionPtr = actionBytes)
            fixed (byte* keyPtr = keyBytes)
            fixed (byte* configPtr = configBytes)
            fixed (byte* dataPtr = dataBytes)
            {
                var resultPtr = StatsigFFI.__internal__test_persistent_storage(
                    storage.Reference,
                    actionPtr,
                    keyPtr,
                    configPtr,
                    dataPtr);

                if (resultPtr != null)
                {
                    StatsigUtils.ReadStringFromPointer(resultPtr);
                }
            }
        }
    }
}
