using System.Collections.Generic;
using Statsig;

#nullable enable

namespace Statsig.Tests
{
    public class MockDataStore : DataStore
    {
        public bool InitializeCalled { get; private set; }
        public bool ShutdownCalled { get; private set; }
        public string? GetCall { get; private set; }
        public SetCall? SetCallInfo { get; private set; }
        public string? SupportPollingUpdatesForCall { get; private set; }
        public bool ShouldReturnPolling { get; set; }
        public DataStoreResponse? NextGetResponse { get; set; } = new DataStoreResponse("test", 123);
        public IList<SetCall> SetCalls { get; } = [];

        public override void Initialize()
        {
            InitializeCalled = true;
        }

        public override void Shutdown()
        {
            ShutdownCalled = true;
        }

        public override DataStoreResponse? Get(string key)
        {
            GetCall = key;
            return NextGetResponse;
        }

        public override void Set(string key, string value, ulong? time = null)
        {
            var call = new SetCall(key, value, time);
            SetCalls.Add(call);
            SetCallInfo = call;
        }

        public override bool SupportPollingUpdatesFor(string key)
        {
            SupportPollingUpdatesForCall = key;
            return ShouldReturnPolling;
        }

        public sealed record SetCall(string Key, string Value, ulong? Time);
    }
}
