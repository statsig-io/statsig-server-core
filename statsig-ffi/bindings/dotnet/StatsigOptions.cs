using System;

namespace StatsigServer
{
    public class StatsigOptions : IDisposable
    {
        private Ref _ref = StatsigFFI.statsig_options_create();

        internal Ref Reference => _ref;

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
                if (_ref.pointer == 0)
                {
                    return;
                }

                fixed (Ref* pRef = &_ref)
                {
                    StatsigFFI.ref_release(pRef);
                    Console.WriteLine("Just After" + _ref.pointer);
                }

                Console.WriteLine("After" + _ref.pointer);
            }
        }
    }
}