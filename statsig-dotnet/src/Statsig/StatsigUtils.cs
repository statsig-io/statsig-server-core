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
            if (value == null)
            {
                return IntPtr.Zero;
            }

#if NET8_0_OR_GREATER
            return Marshal.StringToCoTaskMemUTF8(value);
#else
            // For .NET Framework 4.7.1, manually convert to UTF-8 and allocate memory
            var utf8Bytes = Encoding.UTF8.GetBytes(value);
            var ptr = Marshal.AllocCoTaskMem(utf8Bytes.Length + 1); // +1 for null terminator
            Marshal.Copy(utf8Bytes, 0, ptr, utf8Bytes.Length);
            Marshal.WriteByte(ptr, utf8Bytes.Length, 0); // null terminator
            return ptr;
#endif
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