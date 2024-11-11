using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Statsig
{
    internal static class StatsigUtils
    {
        internal static unsafe string ReadStringFromPointer(byte* pointer)
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
             return Encoding.UTF8.GetString(responseBytes);
         }
    }
}