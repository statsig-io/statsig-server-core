using System;
using System.Runtime.InteropServices;
using System.Text;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IDataStore
    {
        void Initialize();
        void Shutdown();
        DataStoreResponse? Get(string key);
        void Set(string key, string value, ulong? time = null);
        bool SupportPollingUpdatesFor(string key);
    }

    public class DataStoreResponse
    {
        [JsonProperty("result")] public string? Result { get; set; }
        [JsonProperty("time")] public ulong? Time { get; set; }

        public DataStoreResponse(string? result = null, ulong? time = null)
        {
            Result = result;
            Time = time;
        }
    }

    public abstract class DataStore : IDataStore, IDisposable
    {
        private readonly StatsigFFI.data_store_initialize_fn_delegate _initializeDelegate;
        private readonly StatsigFFI.data_store_shutdown_fn_delegate _shutdownDelegate;
        private readonly StatsigFFI.data_store_get_fn_delegate _getDelegate;
        private readonly StatsigFFI.data_store_set_fn_delegate _setDelegate;
        private readonly StatsigFFI.data_store_support_polling_updates_for_fn_delegate _supportPollingDelegate;

        private readonly object _getResultLock = new();
        private GCHandle _pinnedGetResultHandle;
        private bool _hasPinnedGetResult;
        private bool _disposed;

        internal ulong Reference { get; private set; }

        public unsafe DataStore()
        {
            _initializeDelegate = InitializeNative;
            _shutdownDelegate = ShutdownNative;
            _getDelegate = GetNative;
            _setDelegate = SetNative;
            _supportPollingDelegate = SupportPollingNative;

            Reference = StatsigFFI.data_store_create(
                _initializeDelegate,
                _shutdownDelegate,
                _getDelegate,
                _setDelegate,
                _supportPollingDelegate);

            if (Reference == 0)
            {
                Console.Error.WriteLine("[Statsig] Failed to register data store with the native bridge.");
            }
        }

        ~DataStore()
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

            if (_hasPinnedGetResult)
            {
                _pinnedGetResultHandle.Free();
                _hasPinnedGetResult = false;
            }

            if (Reference != 0)
            {
                StatsigFFI.data_store_release(Reference);
                Reference = 0;
            }

            _disposed = true;
        }

        public virtual void Initialize()
        {
        }

        public virtual void Shutdown()
        {
        }

        public virtual DataStoreResponse? Get(string key)
        {
            return null;
        }

        public virtual void Set(string key, string value, ulong? time = null)
        {
        }

        public virtual bool SupportPollingUpdatesFor(string key)
        {
            return false;
        }

        private unsafe void InitializeNative()
        {
            try
            {
                Initialize();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] DataStore.Initialize failed: {ex}");
            }
        }

        private unsafe void ShutdownNative()
        {
            try
            {
                Shutdown();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] DataStore.Shutdown failed: {ex}");
            }
        }

        private unsafe byte* GetNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                var key = Utf8PointerToString(argsPtr, argsLength);
                if (key == null)
                {
                    return null;
                }

                var response = Get(key);
                if (response == null)
                {
                    return null;
                }

                var json = JsonConvert.SerializeObject(response);
                return WriteUtf8ToPinnedBuffer(json);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] DataStore.Get failed: {ex}");
                return null;
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private unsafe void SetNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                DataStoreSetArgs? args = DeserializeFromPointer<DataStoreSetArgs>(argsPtr, argsLength);
                if (args == null || args.Key == null || args.Value == null)
                {
                    return;
                }

                Set(args.Key, args.Value, args.Time);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] DataStore.Set failed: {ex}");
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private unsafe bool SupportPollingNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                string key = Utf8PointerToString(argsPtr, argsLength);
                if (string.IsNullOrEmpty(key))
                {
                    return false;
                }

                return SupportPollingUpdatesFor(key);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] DataStore.SupportPollingUpdatesFor failed: {ex}");
                return false;
            }
            finally
            {
                if (argsPtr != null)
                {
                    StatsigFFI.free_string(argsPtr);
                }
            }
        }

        private unsafe byte* WriteUtf8ToPinnedBuffer(string value)
        {
            var bytes = Encoding.UTF8.GetBytes(value + "\0");
            lock (_getResultLock)
            {
                if (_hasPinnedGetResult)
                {
                    _pinnedGetResultHandle.Free();
                    _hasPinnedGetResult = false;
                }

                _pinnedGetResultHandle = GCHandle.Alloc(bytes, GCHandleType.Pinned);
                _hasPinnedGetResult = true;

                return (byte*)_pinnedGetResultHandle.AddrOfPinnedObject();
            }
        }

        private static unsafe string? Utf8PointerToString(byte* ptr, ulong length)
        {
            if (ptr == null || length == 0)
            {
                return null;
            }

#if NET8_0_OR_GREATER
            return Encoding.UTF8.GetString(new ReadOnlySpan<byte>(ptr, (int)length));
#else
            var len = (int)length;
            var buffer = new byte[len];
            Marshal.Copy((IntPtr)ptr, buffer, 0, len);
            return Encoding.UTF8.GetString(buffer);
#endif
        }

        private static unsafe T? DeserializeFromPointer<T>(byte* ptr, ulong length)
        {
            var json = Utf8PointerToString(ptr, length);
            if (string.IsNullOrEmpty(json))
            {
                return default;
            }

            return JsonConvert.DeserializeObject<T?>(json);
        }

        private sealed class DataStoreSetArgs
        {
            [JsonProperty("key")] public string? Key { get; set; }
            [JsonProperty("value")] public string? Value { get; set; }
            [JsonProperty("time")] public ulong? Time { get; set; }
        }
    }
}
