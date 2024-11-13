using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Statsig
{
    internal static class NativeLibraryLoader
    {
        private const string NativeLibName = "libstatsig_ffi";

        private static readonly Lazy<bool> IsLoaded = new Lazy<bool>(() =>
        {
            NativeLibrary.SetDllImportResolver(typeof(NativeLibraryLoader).Assembly, ResolveNativeLibrary);
            return true;
        });

        public static void EnsureLoaded()
        {
            var _ = IsLoaded.Value;
        }

        private static IntPtr ResolveNativeLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
        {
            if (libraryName == NativeLibName)
            {
                string assemblyLocation = Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location);
                string path = Path.Combine(assemblyLocation, "runtimes", GetRuntimeIdentifier(), "native", $"{NativeLibName}.{GetLibraryExtension()}");

                Console.WriteLine($"Attempting to load library from path: {path}");
                if (File.Exists(path))
                {
                    return NativeLibrary.Load(path, assembly, searchPath);
                }
                else
                {
                    // TODO logger
                    throw new DllNotFoundException($"Could not load {libraryName} from path: {path}");
                }
            }

            return IntPtr.Zero;
        }

        private static string GetLibraryExtension()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) return "dylib";
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) return "dll";
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux)) return "so";

            throw new PlatformNotSupportedException("Unsupported platform.");
        }
        
        private static string GetRuntimeIdentifier()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                return RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "osx-arm64" : "osx-x64";
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                return RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "linux-x64" :
                       RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "linux-arm64" : throw new PlatformNotSupportedException("Unsupported Linux architecture.");
            }
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return RuntimeInformation.ProcessArchitecture == Architecture.X64 ? "win-x64" :
                       RuntimeInformation.ProcessArchitecture == Architecture.X86 ? "win-x86" : throw new PlatformNotSupportedException("Unsupported Windows architecture.");
            }

            throw new PlatformNotSupportedException("Unsupported platform or architecture.");
        }
    }
}
