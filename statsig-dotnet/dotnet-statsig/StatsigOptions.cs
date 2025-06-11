using System;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the Statsig Server SDK
    /// </summary>
    public class StatsigOptions : IDisposable
    {
        private unsafe ulong _ref;
        internal unsafe ulong Reference => _ref;

        public StatsigOptions()
        {
            unsafe
            {
                _ref = StatsigFFI.statsig_options_create(null, null, 0, 0, null, -1, -1, -1, null, -1, -1, -1, -1);
            }
        }

        ~StatsigOptions()
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
            StatsigFFI.statsig_options_release(_ref);
        }
    }
}