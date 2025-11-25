using System.Runtime.InteropServices;

namespace Statsig
{
    internal static unsafe partial class StatsigFFI
    {
#pragma warning disable SYSLIB1054
        [DllImport(__DllName, EntryPoint = "__internal__test_persistent_storage", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern byte* __internal__test_persistent_storage(ulong storageRef, byte* action, byte* key, byte* configName, byte* data);
    }
}
