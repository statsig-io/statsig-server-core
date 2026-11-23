using System;
using System.Diagnostics;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Threading;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Xunit;

#nullable enable

namespace Statsig.Tests
{
    // ------------------------------------------------------------------ [ Bridge + Options ]
    // These tests never create a Statsig instance, so they don't touch the
    // process-global output logger and can run in parallel with everything else.

    public class OutputLoggerProviderBridgeTests
    {
        [Theory]
        [InlineData("debug", OutputLogLevel.Debug)]
        [InlineData("info", OutputLogLevel.Info)]
        [InlineData("warn", OutputLogLevel.Warn)]
        [InlineData("error", OutputLogLevel.Error)]
        public void LogCallsAreRoutedWithLevelTagAndMessage(string action, OutputLogLevel expectedLevel)
        {
            using var provider = new MockOutputLoggerProvider();

            CallProvider(provider, action, "StatsigHttpSpecsAdapter", "Background specs sync failed");

            var call = Assert.Single(provider.Logs);
            Assert.Equal(expectedLevel, call.Level);
            Assert.Equal("StatsigHttpSpecsAdapter", call.Tag);
            Assert.Equal("Background specs sync failed", call.Message);
        }

        [Fact]
        public void InitAndShutdownAreRouted()
        {
            using var provider = new MockOutputLoggerProvider();

            CallProvider(provider, "init", "", "");
            CallProvider(provider, "shutdown", "", "");

            Assert.Equal(1, provider.InitCount);
            Assert.Equal(1, provider.ShutdownCount);
            Assert.Empty(provider.Logs);
        }

        [Fact]
        public void UnicodeAndEmptyStringsAreDecoded()
        {
            using var provider = new MockOutputLoggerProvider();

            CallProvider(provider, "warn", "Tag✓", "héllo — 世界 🚀");
            CallProvider(provider, "info", "", "");

            var calls = provider.Logs.ToArray();
            Assert.Equal("Tag✓", calls[0].Tag);
            Assert.Equal("héllo — 世界 🚀", calls[0].Message);
            Assert.Equal("", calls[1].Tag);
            Assert.Equal("", calls[1].Message);
        }

        [Fact]
        public void ExceptionsInUserCodeDoNotEscapeToNativeCode()
        {
            using var provider = new MockOutputLoggerProvider { ThrowOnLog = true };

            CallProvider(provider, "error", "Tag", "first");

            provider.ThrowOnLog = false;
            CallProvider(provider, "error", "Tag", "second");

            var call = Assert.Single(provider.Logs);
            Assert.Equal("second", call.Message);
        }

        [Fact]
        public void DisposeReleasesTheNativeReference()
        {
            var provider = new MockOutputLoggerProvider();
            Assert.NotEqual(0UL, provider.Reference);

            provider.Dispose();
            provider.Dispose(); // idempotent

            Assert.Equal(0UL, provider.Reference);
        }

        internal static unsafe void CallProvider(OutputLoggerProvider provider, string action, string tag, string message)
        {
            var actionBytes = StatsigUtils.ToUtf8NullTerminated(action);
            var tagBytes = StatsigUtils.ToUtf8NullTerminated(tag);
            var messageBytes = StatsigUtils.ToUtf8NullTerminated(message);

            fixed (byte* actionPtr = actionBytes)
            fixed (byte* tagPtr = tagBytes)
            fixed (byte* messagePtr = messageBytes)
            {
                StatsigFFI.__internal__test_output_logger_provider(provider.Reference, actionPtr, tagPtr, messagePtr);
            }
        }
    }

    public class OutputLoggerOptionsTests
    {
        [Theory]
        [InlineData(OutputLogLevel.None, "none")]
        [InlineData(OutputLogLevel.Error, "error")]
        [InlineData(OutputLogLevel.Warn, "warn")]
        [InlineData(OutputLogLevel.Info, "info")]
        [InlineData(OutputLogLevel.Debug, "debug")]
        public void SetOutputLogLevelSerializesTheCoreLevelString(OutputLogLevel level, string expected)
        {
            var builder = new StatsigOptionsBuilder();

            var result = builder.SetOutputLogLevel(level);

            Assert.Same(builder, result);
            Assert.Equal(expected, builder.outputLogLevel);
            Assert.Contains($"\"output_log_level\":\"{expected}\"", Serialize(builder));
        }

        [Fact]
        public void OutputLogLevelIsOmittedWhenNotSet()
        {
            Assert.DoesNotContain("output_log_level", Serialize(new StatsigOptionsBuilder()));
        }

        [Fact]
        public void UnknownOutputLogLevelIsOmittedSoTheSdkDefaultApplies()
        {
            var builder = new StatsigOptionsBuilder().SetOutputLogLevel((OutputLogLevel)99);

            Assert.Null(builder.outputLogLevel);
            Assert.DoesNotContain("output_log_level", Serialize(builder));
        }

        [Fact]
        public void SetOutputLoggerProviderIsAttachedToOptions()
        {
            using var provider = new MockOutputLoggerProvider();
            var builder = new StatsigOptionsBuilder();

            var result = builder.SetOutputLoggerProvider(provider);
            using var options = builder.Build();

            Assert.Same(builder, result);
            Assert.Same(provider, options.OutputLoggerProvider);
            Assert.NotEqual(0UL, options.Reference);
            // The ref is only set while building, so it can't leak into a second Build().
            Assert.Null(builder.outputLoggerProviderRef);
        }

        [Fact]
        public void ProviderRefIsSerializedForTheNativeLayer()
        {
            using var provider = new MockOutputLoggerProvider();
            var builder = new StatsigOptionsBuilder().SetOutputLoggerProvider(provider);
            builder.outputLoggerProviderRef = provider.Reference;

            Assert.Contains($"\"output_logger_provider_ref\":{provider.Reference}", Serialize(builder));
        }

        [Fact]
        public void DisposedProviderIsNotAttached()
        {
            var provider = new MockOutputLoggerProvider();
            provider.Dispose();

            using var options = new StatsigOptionsBuilder().SetOutputLoggerProvider(provider).Build();

            Assert.Null(options.OutputLoggerProvider);
        }

        private static string Serialize(StatsigOptionsBuilder builder) =>
            JsonConvert.SerializeObject(builder, new JsonSerializerSettings { NullValueHandling = NullValueHandling.Ignore });
    }

    // ------------------------------------------------------------------ [ End to end ]
    // The native output logger is process-global and the first Statsig instance to
    // initialize wins, so these tests must not overlap with each other or any other
    // test that creates a Statsig instance.

    [CollectionDefinition(nameof(OutputLoggerCollection), DisableParallelization = true)]
    public class OutputLoggerCollection { }

    [Collection(nameof(OutputLoggerCollection))]
    public class OutputLoggerEndToEndTests
    {
        // Nothing listens here, so every specs download fails fast - the same code
        // path that produces the customer's "Background specs sync failed" line.
        private const string UnreachableUrl = "http://127.0.0.1:1/v2/download_config_specs";
        private static readonly TimeSpan WaitTimeout = TimeSpan.FromSeconds(20);

        [Fact]
        public async Task DebugLevelForwardsSdkLogsWithTagsAndLifecycle()
        {
            var provider = new MockOutputLoggerProvider();

            await RunStatsig(provider, OutputLogLevel.Debug, () =>
                provider.HasLevel(OutputLogLevel.Debug) && provider.Logs.Any(IsBackgroundSyncFailure));

            Assert.Equal(1, provider.InitCount);
            // The core calls Shutdown from both Statsig.Shutdown() and when the instance is released.
            Assert.True(provider.ShutdownCount >= 1, $"shutdown={provider.ShutdownCount}");
            Assert.Contains(provider.Logs, l => l.Level == OutputLogLevel.Debug);
            // The exact line from the customer report, delivered with its level and tag.
            Assert.Contains(provider.Logs, IsBackgroundSyncFailure);
            Assert.All(provider.Logs, l => Assert.False(string.IsNullOrEmpty(l.Tag)));
        }

        [Fact]
        public async Task WarnLevelFiltersDebugAndInfo()
        {
            var provider = new MockOutputLoggerProvider();

            await RunStatsig(provider, OutputLogLevel.Warn, () => provider.Logs.Any(IsBackgroundSyncFailure));

            Assert.Contains(provider.Logs, IsBackgroundSyncFailure);
            Assert.DoesNotContain(provider.Logs, l => l.Level == OutputLogLevel.Debug || l.Level == OutputLogLevel.Info);
        }

        [Fact]
        public async Task NoneLevelSuppressesLeveledLogsIncludingBackgroundSyncFailures()
        {
            var provider = new MockOutputLoggerProvider();

            // Long enough for several 1s background syncs to fail (see the Debug test).
            await RunStatsig(provider, OutputLogLevel.None, () => false, TimeSpan.FromSeconds(4));

            Assert.Equal(1, provider.InitCount);
            Assert.DoesNotContain(provider.Logs, IsBackgroundSyncFailure);
            // Known core behaviour: errors raised via log_error_to_statsig_and_console!
            // (e.g. "Initialization Error") bypass the level check in every binding.
            Assert.True(provider.Logs.All(l => l.Level == OutputLogLevel.Error), Describe(provider));
        }

        [Fact]
        public async Task ProviderIsNeverCollectedWhileTheNativeCoreCanStillCallIt()
        {
            // Instance A registers a provider, then is shut down and disposed and every
            // managed reference to the provider is dropped. The native core only holds
            // raw function pointers (invisible to the GC), so without rooting the
            // provider and its delegates would be collected here.
            var weakProvider = await RunAndDropInstanceWithProvider();
            ForceFullGc();
            Assert.True(weakProvider.IsAlive, "provider must stay rooted while the native core can call it");

            // Instance B sets no provider and keeps logging (including background sync
            // failures) after another full GC. Whatever the core routes to, the test host
            // must not crash with "callback was made on a garbage collected delegate".
            using (var options = new StatsigOptionsBuilder()
                       .SetSpecsURL(UnreachableUrl)
                       .SetLogEventURL(UnreachableUrl)
                       .SetSpecsSyncIntervalMs(1000)
                       .SetInitTimeoutMs(1000)
                       .Build())
            using (var statsigB = new Statsig("secret-output-logger-gc-b", options))
            {
                await statsigB.Initialize();
                ForceFullGc();
                await Task.Delay(TimeSpan.FromSeconds(2)); // at least one background sync
                await statsigB.Shutdown();
            }

            Assert.True(weakProvider.IsAlive);
        }

        [MethodImpl(MethodImplOptions.NoInlining)]
        private static async Task<WeakReference> RunAndDropInstanceWithProvider()
        {
            var provider = new MockOutputLoggerProvider();
            await RunStatsig(provider, OutputLogLevel.Debug, () => !provider.Logs.IsEmpty);
            Assert.False(provider.Logs.IsEmpty);
            return new WeakReference(provider);
        }

        private static async Task RunStatsig(
            MockOutputLoggerProvider provider,
            OutputLogLevel level,
            Func<bool> until,
            TimeSpan? timeout = null)
        {
            ForceFullGc(); // finalize Statsig instances leaked by other tests first

            using var options = new StatsigOptionsBuilder()
                .SetOutputLoggerProvider(provider)
                .SetOutputLogLevel(level)
                .SetSpecsURL(UnreachableUrl)
                .SetLogEventURL(UnreachableUrl)
                .SetSpecsSyncIntervalMs(1000)
                .SetInitTimeoutMs(1000)
                .Build();
            using var statsig = new Statsig("secret-output-logger-e2e", options);

            await statsig.Initialize();
            await WaitFor(until, timeout ?? WaitTimeout);
            await statsig.Shutdown();
        }

        private static bool IsBackgroundSyncFailure(MockOutputLoggerProvider.LogCall l) =>
            l.Level == OutputLogLevel.Error &&
            l.Tag == "StatsigHttpSpecsAdapter" &&
            l.Message.StartsWith("Background specs sync failed");

        private static string Describe(MockOutputLoggerProvider p) =>
            string.Join("\n", p.Logs.Select(l => $"{l.Level} [{l.Tag}] {l.Message}"));

        private static void ForceFullGc()
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
        }

        private static async Task WaitFor(Func<bool> condition, TimeSpan timeout)
        {
            var stopwatch = Stopwatch.StartNew();
            while (!condition() && stopwatch.Elapsed < timeout)
            {
                await Task.Delay(50);
            }
        }
    }
}
