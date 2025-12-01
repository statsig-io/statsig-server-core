using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Newtonsoft.Json;

namespace Statsig
{
    public interface IPersistentStorage
    {
        IDictionary<string, StickyValues> Load(string key);
        void Save(string key, string configName, StickyValues data);
        void Delete(string key, string configName);
    }

    public abstract class PersistentStorage : IPersistentStorage, IDisposable
    {
        private readonly StatsigFFI.persistent_storage_create_load_fn_delegate _loadDelegate;
        private readonly StatsigFFI.persistent_storage_create_save_fn_delegate _saveDelegate;
        private readonly StatsigFFI.persistent_storage_create_delete_fn_delegate _deleteDelegate;
        private bool _disposed;
        public unsafe ulong _ref;

        internal ulong Reference => _ref;

        public unsafe PersistentStorage()
        {
            _loadDelegate = LoadNative;
            _saveDelegate = SaveNative;
            _deleteDelegate = DeleteNative;

            byte[] nameBytes = System.Text.Encoding.UTF8.GetBytes("dotnet\0");
            fixed (byte* namePtr = nameBytes)
            {
                _ref = StatsigFFI.persistent_storage_create(namePtr, _loadDelegate, _saveDelegate, _deleteDelegate);
                if (_ref == 0)
                {
                    Console.Error.WriteLine("[Statsig] Failed to register persistent storage with the native bridge.");
                    return;
                }
            }
        }

        ~PersistentStorage()
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

            if (_ref != 0)
            {
                StatsigFFI.persistent_storage_release(_ref);
                _ref = 0;
            }

            _disposed = true;
        }

        // Implement the IPersistentStorage interface
        public virtual IDictionary<string, StickyValues> Load(string key)
        {
            // User should override this
            return null;
        }

        public virtual void Save(string key, string configName, StickyValues data)
        {
            // User should override this
        }

        public virtual void Delete(string key, string configName)
        {
            // User should override this
        }

        // FFI callback implementations
        private unsafe byte* LoadNative(byte* keyPtr, ulong keyLength)
        {
            try
            {
                var key = StatsigUtils.ReadStringFromPointer(keyPtr);
                if (string.IsNullOrEmpty(key))
                {
                    return null;
                }

                var result = Load(key);
                if (result == null || result.Count == 0)
                {
                    return null;
                }

                var json = JsonConvert.SerializeObject(result);
                var jsonBytes = System.Text.Encoding.UTF8.GetBytes(json + "\0");
                var buffer = Marshal.AllocCoTaskMem(jsonBytes.Length);
                Marshal.Copy(jsonBytes, 0, buffer, jsonBytes.Length);
                return (byte*)buffer;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] PersistentStorage.Load failed: {ex}");
                return null;
            }
        }

        private unsafe void SaveNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                var argsJson = StatsigUtils.ReadStringFromPointer(argsPtr);
                if (string.IsNullOrEmpty(argsJson))
                {
                    return;
                }

                var args = JsonConvert.DeserializeObject<Dictionary<string, object>>(argsJson);
                if (args == null ||
                    !args.TryGetValue("key", out var keyValue) ||
                    !args.TryGetValue("config_name", out var configValue) ||
                    !args.TryGetValue("data", out var dataValue))
                {
                    return;
                }

                var key = keyValue?.ToString();
                var configName = configValue?.ToString();
                var dataJson = dataValue?.ToString();

                if (string.IsNullOrEmpty(key) || string.IsNullOrEmpty(configName) || string.IsNullOrEmpty(dataJson))
                {
                    return;
                }

                var data = JsonConvert.DeserializeObject<StickyValues>(dataJson);
                if (data == null)
                {
                    return;
                }

                Save(key, configName, data);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] PersistentStorage.Save failed: {ex}");
            }
        }

        private unsafe void DeleteNative(byte* argsPtr, ulong argsLength)
        {
            try
            {
                var argsJson = StatsigUtils.ReadStringFromPointer(argsPtr);
                if (string.IsNullOrEmpty(argsJson))
                {
                    return;
                }

                var args = JsonConvert.DeserializeObject<Dictionary<string, object>>(argsJson);
                if (args == null ||
                    !args.TryGetValue("key", out var keyValue) ||
                    !args.TryGetValue("config_name", out var configValue))
                {
                    return;
                }

                var key = keyValue?.ToString();
                var configName = configValue?.ToString();

                if (string.IsNullOrEmpty(key) || string.IsNullOrEmpty(configName))
                {
                    return;
                }

                Delete(key, configName);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Statsig] PersistentStorage.Delete failed: {ex}");
            }
        }
    }
}
