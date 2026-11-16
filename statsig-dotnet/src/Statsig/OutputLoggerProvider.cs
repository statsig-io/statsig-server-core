using System;
using System.Collections.Generic;

namespace Statsig
{
    /// <summary>
    /// Minimum severity of messages the SDK writes to its output logger.
    /// Messages below the configured level are dropped before reaching
    /// the console or a custom <see cref="OutputLoggerProvider"/>.
    /// </summary>
    public enum OutputLogLevel
    {
        None = 0,
        Error = 1,
        Warn = 2,
        Info = 3,
        Debug = 4,
    }

    public interface IOutputLoggerProvider
    {
        void Init();
        void Debug(string tag, string message);
        void Info(string tag, string message);
        void Warn(string tag, string message);
        void Error(string tag, string message);
        void Shutdown();
    }

    /// <summary>
    /// Receives the SDK's internal log messages (for example to forward them to
    /// Microsoft.Extensions.Logging). Register it with
    /// <see cref="StatsigOptionsBuilder.SetOutputLoggerProvider"/>.
    /// </summary>
    /// <remarks>
    /// <list type="bullet">
    /// <item>Methods are called synchronously from SDK background threads, so
    /// implementations must be thread-safe and fast.</item>
    /// <item>Do not call Statsig APIs from inside these methods.</item>
    /// <item>The SDK keeps a single output logger per process: the first Statsig
    /// instance to initialize decides which provider is active.</item>
    /// <item>Only messages at or above the level set with
    /// <see cref="StatsigOptionsBuilder.SetOutputLogLevel"/> are delivered
    /// (default <see cref="OutputLogLevel.Warn"/>).</item>
    /// <item>Messages longer than 400 characters are truncated and SDK keys are masked.</item>
    /// <item><see cref="Shutdown"/> may be called more than once.</item>
    /// <item>Once passed to <see cref="StatsigOptionsBuilder.SetOutputLoggerProvider"/>,
    /// the provider is kept alive for the rest of the process, because the native
    /// SDK core can still call it. Call <c>Statsig.Shutdown()</c> before the process
    /// exits.</item>
    /// </list>
    /// </remarks>
    public abstract class OutputLoggerProvider : IOutputLoggerProvider, IDisposable
    {
        // The native core keeps the active provider in process-global state and may
        // call it after the Statsig instance and options are gone. Providers handed
        // to StatsigOptions are therefore kept reachable for the life of the process
        // so their native callbacks can never be garbage collected.
        private static readonly object RootLock = new();
        private static readonly HashSet<OutputLoggerProvider> RootedProviders = new();

        private readonly StatsigFFI.output_logger_provider_init_fn_delegate _initDelegate;
        private readonly StatsigFFI.output_logger_provider_log_fn_delegate _logDelegate;
        private readonly StatsigFFI.output_logger_provider_shutdown_fn_delegate _shutdownDelegate;

        private volatile bool _disposed;

        internal ulong Reference { get; private set; }

        protected OutputLoggerProvider()
        {
            unsafe
            {
                _initDelegate = InitNative;
                _logDelegate = LogNative;
                _shutdownDelegate = ShutdownNative;
            }

            Reference = StatsigFFI.output_logger_provider_create(
                _initDelegate,
                _logDelegate,
                _shutdownDelegate);

            if (Reference == 0)
            {
                Console.Error.WriteLine("[Statsig] Failed to register output logger provider with the native bridge.");
            }
        }

        ~OutputLoggerProvider()
        {
            Dispose(false);
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (_disposed)
            {
                return;
            }

            // Native callbacks become no-ops from here on.
            _disposed = true;

            if (Reference != 0)
            {
                StatsigFFI.output_logger_provider_release(Reference);
                Reference = 0;
            }
        }

        internal void RootForProcessLifetime()
        {
            lock (RootLock)
            {
                RootedProviders.Add(this);
            }
        }

        public virtual void Init()
        {
        }

        public virtual void Debug(string tag, string message)
        {
        }

        public virtual void Info(string tag, string message)
        {
        }

        public virtual void Warn(string tag, string message)
        {
        }

        public virtual void Error(string tag, string message)
        {
        }

        public virtual void Shutdown()
        {
        }

        private void InitNative()
        {
            if (_disposed)
            {
                return;
            }

            try
            {
                Init();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] OutputLoggerProvider.Init failed: {ex}");
            }
        }

        private unsafe void LogNative(uint level, byte* tagPtr, ulong tagLength, byte* messagePtr, ulong messageLength)
        {
            if (_disposed)
            {
                return;
            }

            try
            {
                // Buffers are borrowed from the native core: copy, never free.
                var tag = StatsigUtils.Utf8PointerToString(tagPtr, tagLength) ?? string.Empty;
                var message = StatsigUtils.Utf8PointerToString(messagePtr, messageLength) ?? string.Empty;

                switch ((OutputLogLevel)level)
                {
                    case OutputLogLevel.Error:
                        Error(tag, message);
                        break;
                    case OutputLogLevel.Warn:
                        Warn(tag, message);
                        break;
                    case OutputLogLevel.Info:
                        Info(tag, message);
                        break;
                    case OutputLogLevel.Debug:
                        Debug(tag, message);
                        break;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] OutputLoggerProvider log callback failed: {ex}");
            }
        }

        private void ShutdownNative()
        {
            if (_disposed)
            {
                return;
            }

            try
            {
                Shutdown();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] OutputLoggerProvider.Shutdown failed: {ex}");
            }
        }
    }
}
