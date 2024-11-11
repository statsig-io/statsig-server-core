using System;

namespace Statsig
{
    /// <summary>
    /// Configuration options for the Statsig Server SDK
    /// </summary>
    public class StatsigOptions : IDisposable
    {
        private unsafe byte* _ref;
        internal unsafe byte* Reference => _ref;

        public StatsigOptions()
        {
            unsafe
            {
                _ref = StatsigFFI.statsig_options_create(null, null, null);
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
            unsafe
            {
                if (_ref == null)
                {
                    return;
                }

                StatsigFFI.statsig_options_release(_ref);
                _ref = null;
            }
        }
    }
}