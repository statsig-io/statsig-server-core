using System.Runtime.InteropServices;

namespace Statsig
{
    internal static unsafe partial class StatsigFFI
    {
#pragma warning disable SYSLIB1054
        [DllImport(__DllName, EntryPoint = "__internal__test_persistent_storage", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* __internal__test_persistent_storage(ulong storageRef, byte* action, byte* key, byte* configName, byte* data);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void data_store_initialize_fn_delegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void data_store_shutdown_fn_delegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate byte* data_store_get_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void data_store_set_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.U1)]
        internal delegate bool data_store_support_polling_updates_for_fn_delegate(byte* argsPtr, ulong argsLength);

        [DllImport(__DllName, EntryPoint = "data_store_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong data_store_create(
            data_store_initialize_fn_delegate initializeFn,
            data_store_shutdown_fn_delegate shutdownFn,
            data_store_get_fn_delegate getFn,
            data_store_set_fn_delegate setFn,
            data_store_support_polling_updates_for_fn_delegate supportPollingUpdatesForFn);

        [DllImport(__DllName, EntryPoint = "data_store_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void data_store_release(ulong dataStoreRef);

        [DllImport(__DllName, EntryPoint = "__internal__test_data_store", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* __internal__test_data_store(ulong dataStoreRef, byte* path, byte* value);
    }
}
