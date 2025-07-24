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
            result[^1] = 0; // null terminator
            return result;
        }
    }
}