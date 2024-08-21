using System;
using System.Text;

namespace StatsigServer
{
    public class StatsigUser : IDisposable
    {
        private Ref _ref;

        internal Ref Reference => _ref;

        public StatsigUser(string userId, string email)
        {
            var userIdBytes = Encoding.UTF8.GetBytes(userId);
            var emailBytes = Encoding.UTF8.GetBytes(email);
            unsafe
            {
                fixed (byte* userIdPtr = userIdBytes)
                fixed (byte* emailPtr = emailBytes)
                {
                    _ref = StatsigFFI.statsig_user_create(userIdPtr, emailPtr);
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