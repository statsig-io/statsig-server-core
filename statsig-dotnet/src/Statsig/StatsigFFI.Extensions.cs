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

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void observability_client_init_fn_delegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void observability_client_increment_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void observability_client_gauge_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void observability_client_dist_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void observability_client_error_fn_delegate(byte* argsPtr, ulong argsLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.U1)]
        internal delegate bool observability_client_should_enable_high_cardinality_for_this_tag_fn_delegate(byte* argsPtr, ulong argsLength);

        [DllImport(__DllName, EntryPoint = "observability_client_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong observability_client_create(
            observability_client_init_fn_delegate initFn,
            observability_client_increment_fn_delegate incrementFn,
            observability_client_gauge_fn_delegate gaugeFn,
            observability_client_dist_fn_delegate distFn,
            observability_client_error_fn_delegate errorFn,
            observability_client_should_enable_high_cardinality_for_this_tag_fn_delegate shouldEnableHighCardinalityForThisTagFn);

        [DllImport(__DllName, EntryPoint = "observability_client_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void observability_client_release(ulong observabilityClientRef);

        [DllImport(__DllName, EntryPoint = "__internal__test_observability_client", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void __internal__test_observability_client(ulong observabilityClientRef, byte* action, byte* metricName, double value, byte* tags);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void output_logger_provider_init_fn_delegate();

        // tagPtr / messagePtr are borrowed, non-null-terminated UTF-8 buffers owned by the native core.
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void output_logger_provider_log_fn_delegate(uint level, byte* tagPtr, ulong tagLength, byte* messagePtr, ulong messageLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void output_logger_provider_shutdown_fn_delegate();

        [DllImport(__DllName, EntryPoint = "output_logger_provider_create", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ulong output_logger_provider_create(
            output_logger_provider_init_fn_delegate initFn,
            output_logger_provider_log_fn_delegate logFn,
            output_logger_provider_shutdown_fn_delegate shutdownFn);

        [DllImport(__DllName, EntryPoint = "output_logger_provider_release", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void output_logger_provider_release(ulong outputLoggerProviderRef);

        [DllImport(__DllName, EntryPoint = "__internal__test_output_logger_provider", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void __internal__test_output_logger_provider(ulong outputLoggerProviderRef, byte* action, byte* tag, byte* message);
    }
}
