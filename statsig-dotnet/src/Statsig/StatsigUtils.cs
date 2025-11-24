using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Statsig
{
    internal static class StatsigUtils
    {
        internal static unsafe string? ReadStringFromPointer(byte* pointer)
        {
            if (pointer == null)
            {
                return null;
            }

            var length = 0;
            while (*(pointer + length) != 0)
            {
                length++;
            }

            var responseBytes = new byte[length];
            Marshal.Copy((IntPtr)pointer, responseBytes, 0, length);
            StatsigFFI.free_string(pointer);
            return Encoding.UTF8.GetString(responseBytes);
        }

        internal static byte[] ToUtf8NullTerminated(string value)
        {
            var bytes = Encoding.UTF8.GetBytes(value);
            var result = new byte[bytes.Length + 1];
            Buffer.BlockCopy(bytes, 0, result, 0, bytes.Length);
#if NET8_0_OR_GREATER
            result[^1] = 0; // null terminator
#else
            result[result.Length - 1] = 0; // null terminator
#endif
            return result;
        }

        internal static IntPtr StringToNativeUtf8(string? value)
        {
            return value == null ? IntPtr.Zero : Marshal.StringToCoTaskMemUTF8(value);
        }

        internal static void FreeNativeUtf8(IntPtr ptr)
        {
            if (ptr != IntPtr.Zero)
            {
                Marshal.FreeCoTaskMem(ptr);
            }
        }
    }
}