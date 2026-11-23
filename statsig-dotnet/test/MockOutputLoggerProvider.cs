using System;
using System.Collections.Concurrent;
using System.Linq;
using Statsig;

#nullable enable

namespace Statsig.Tests
{
    public class MockOutputLoggerProvider : OutputLoggerProvider
    {
        public record LogCall(OutputLogLevel Level, string Tag, string Message);

        // Written from native SDK threads while the test thread reads.
        public ConcurrentQueue<LogCall> Logs { get; } = new();
        private int _initCount;
        private int _shutdownCount;

        public int InitCount => _initCount;
        public int ShutdownCount => _shutdownCount;
        public bool ThrowOnLog { get; set; }

        public override void Init() => System.Threading.Interlocked.Increment(ref _initCount);

        public override void Shutdown() => System.Threading.Interlocked.Increment(ref _shutdownCount);

        public override void Debug(string tag, string message) => Record(OutputLogLevel.Debug, tag, message);

        public override void Info(string tag, string message) => Record(OutputLogLevel.Info, tag, message);

        public override void Warn(string tag, string message) => Record(OutputLogLevel.Warn, tag, message);

        public override void Error(string tag, string message) => Record(OutputLogLevel.Error, tag, message);

        public bool HasLevel(OutputLogLevel level) => Logs.Any(l => l.Level == level);

        private void Record(OutputLogLevel level, string tag, string message)
        {
            if (ThrowOnLog)
            {
                throw new InvalidOperationException("boom");
            }

            Logs.Enqueue(new LogCall(level, tag, message));
        }
    }
}
