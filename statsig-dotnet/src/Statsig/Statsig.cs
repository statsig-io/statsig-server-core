using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;

namespace Statsig
{
    public class Statsig : IDisposable
    {
        private unsafe ulong _statsigRef;

        public Statsig(string sdkKey, StatsigOptions options)
        {
            Console.WriteLine($"Operating System: {RuntimeInformation.OSDescription}");
            Console.WriteLine($"Architecture: {RuntimeInformation.OSArchitecture}");
            var sdkKeyBytes = Encoding.UTF8.GetBytes(sdkKey);
            unsafe
            {
                fixed (byte* sdkKeyPtr = sdkKeyBytes)
                {
                    _statsigRef = StatsigFFI.statsig_create(sdkKeyPtr, options.Reference);
                }
            }
        }

        ~Statsig()
        {
            Dispose(false);
        }

        public Task Initialize()
        {
            var source = new TaskCompletionSource<bool>();
            StatsigFFI.statsig_initialize(_statsigRef, () =>
            {
                source.SetResult(true);
            });
            return source.Task;
        }

        unsafe public bool CheckGate(StatsigUser user, string gateName)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            fixed (byte* gateNamePtr = gateNameBytes)
            {
                return StatsigFFI.statsig_check_gate(_statsigRef, user.Reference, gateNamePtr, null);
            }
        }

        unsafe public Experiment GetExperiment(StatsigUser user, string experimentName)
        {
            var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);

            fixed (byte* experimentNamePtr = experimentNameBytes)
            {
                var jsonStringPtr =
                    StatsigFFI.statsig_get_experiment(_statsigRef, user.Reference, experimentNamePtr, null);
                var jsonString = StatsigUtils.ReadStringFromPointer(jsonStringPtr);
                if (jsonString == null)
                {
                    return new Experiment(experimentName, null, null, null, null);
                }
                return JsonConvert.DeserializeObject<Experiment>(jsonString) ??
                       new Experiment(experimentName, null, null, null, null);
            }
        }

        unsafe public string? GetClientInitializeResponse(StatsigUser user)
        {
            if (_statsigRef == 0)
            {
                Console.WriteLine("Failed to get statsig ref");
            }

            if (user.Reference == 0)
            {
                Console.WriteLine("Failed to get user reference");
            }

            return StatsigUtils.ReadStringFromPointer(StatsigFFI.statsig_get_client_init_response(_statsigRef, user.Reference, null));
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (_statsigRef == 0)
            {
                return;
            }
            StatsigFFI.statsig_release(_statsigRef);
        }
    }
}