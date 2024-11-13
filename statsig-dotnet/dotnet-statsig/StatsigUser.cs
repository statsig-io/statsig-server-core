using System;
using System.Text;

namespace Statsig
{
    public class StatsigUser : IDisposable
    {
        private unsafe byte* _ref;

        internal unsafe byte* Reference => _ref;

        public StatsigUser(string userId, string email)
        {
            NativeLibraryLoader.EnsureLoaded();
            var userIdBytes = Encoding.UTF8.GetBytes(userId);
            var emailBytes = Encoding.UTF8.GetBytes(email);
            unsafe
            {
                fixed (byte* userIdPtr = userIdBytes)
                fixed (byte* emailPtr = emailBytes)
                {
                    _ref = StatsigFFI.statsig_user_create(userIdPtr, null, emailPtr
                    , null, null, null, null, null, null, null);
                }
            }
        }

        ~StatsigUser()
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
                StatsigFFI.statsig_user_release(_ref);
            }
        }
    }
}