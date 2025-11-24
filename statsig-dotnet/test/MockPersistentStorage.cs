using System.Collections.Generic;

namespace Statsig.Tests
{
    public class MockPersistentStorage : PersistentStorage
    {
        public IList<string> LoadCalls { get; } = [];
        public IList<SaveCall> SaveCalls { get; } = [];
        public IList<DeleteCall> DeleteCalls { get; } = [];
        private readonly Dictionary<string, Dictionary<string, StickyValues>> storage = [];

        public override IDictionary<string, StickyValues> Load(string key)
        {
            LoadCalls.Add(key);
            return storage.TryGetValue(key, out var values)
                ? new Dictionary<string, StickyValues>(values)
                : [];
        }

        public override void Save(string key, string config_name, StickyValues data)
        {
            SaveCalls.Add(new SaveCall(key, config_name, data));
            if (!storage.TryGetValue(key, out var configs))
            {
                configs = [];
                storage[key] = configs;
            }

            configs[config_name] = data;
        }

        public override void Delete(string key, string config_name)
        {
            DeleteCalls.Add(new DeleteCall(key, config_name));
            if (storage.TryGetValue(key, out var configs))
            {
                configs.Remove(config_name);
                if (configs.Count == 0)
                {
                    storage.Remove(key);
                }
            }
        }

        public sealed record SaveCall(string Key, string ConfigName, StickyValues Data);

        public sealed record DeleteCall(string Key, string ConfigName);
    }
}
