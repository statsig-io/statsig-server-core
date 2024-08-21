using System;
using System.Text;
using System.Threading.Tasks;

namespace StatsigServer
{
    public class Statsig : IDisposable
    {
        private Ref _ref;
        internal Ref Reference => _ref;

        public Statsig(string sdkKey, StatsigOptions options)
        {
            var sdkKeyBytes = Encoding.UTF8.GetBytes(sdkKey);
            unsafe
            {
                fixed (byte* sdkKeyPtr = sdkKeyBytes)
                {
                    _ref = StatsigFFI.statsig_create(sdkKeyPtr, options.Reference);
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
            StatsigFFI.statsig_initialize(_ref, () =>
            {
                source.SetResult(true);
            });

            return source.Task;
        }

        public bool CheckGate(StatsigUser user, string gateName)
        {
            var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
            unsafe
            {
                fixed (byte* gateNamePtr = gateNameBytes)
                {
                    return StatsigFFI.statsig_check_gate(_ref, user.Reference, gateNamePtr);
                }    
            }
            
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
//     public class Experiment
//     {
//         public readonly string Name;
//         public readonly string RuleID;
//         public readonly Dictionary<string, object> Value;
//
//         public Experiment(string name, string ruleID, Dictionary<string, object> value)
//         {
//             Name = name;
//             RuleID = ruleID;
//             Value = value;
//         }
//     }
//
//     public unsafe class User : IDisposable
//     {
//         private Ref _userRef;
//         private bool _disposed;
//
//         public User(string userId, string email)
//         {
//             var userIdBytes = Encoding.UTF8.GetBytes(userId);
//             var emailBytes = Encoding.UTF8.GetBytes(email);
//             fixed (byte* userIdPtr = userIdBytes)
//             fixed (byte* emailPtr = emailBytes)
//             {
//                 _userRef = StatsigFFI.statsig_user_create(userIdPtr, emailPtr);
//             }
//         }
//
//         ~User()
//         {
//             Dispose(false);
//         }
//
//         public void Dispose()
//         {
//             Dispose(true);
//             GC.SuppressFinalize(this);
//         }
//
//         protected virtual void Dispose(bool disposing)
//         {
//             if (!_disposed)
//             {
//                 if (_userRef != null)
//                 {
//                     StatsigFFI.ref_release(_userRef);
//                     _userRef = null;
//                 }
//
//                 _disposed = true;
//             }
//         }
//
//         internal Ref* UserRef => _userRef;
//     }
//
//     public unsafe class StatsigServer : IDisposable
//     {
//         private Ref* _statsigRef;
//         private bool _disposed;
//
//         public static Task<StatsigServer> Create(string sdkKey)
//         {
//             var source = new TaskCompletionSource<StatsigServer>();
//
//             var options = StatsigFFI.statsig_options_create();
//             var sdkKeyBytes = Encoding.UTF8.GetBytes(sdkKey);
//             fixed (byte* sdkKeyPtr = sdkKeyBytes)
//             {
//                 StatsigFFI.statsig_initialize(sdkKeyPtr, options, @ref =>
//                 {
//                     source.SetResult(new StatsigServer(@ref));
//                 });
//             }
//             
//             return source.Task;
//         }
//
//         ~StatsigServer()
//         {
//             Dispose(false);
//         }
//
//         public void Dispose()
//         {
//             Dispose(true);
//             GC.SuppressFinalize(this);
//         }
//
//         protected virtual void Dispose(bool disposing)
//         {
//             if (!_disposed)
//             {
//                 if (_statsigRef != null)
//                 {
//                     StatsigFFI.ref_release(_statsigRef);
//                     _statsigRef = null;
//                 }
//
//                 _disposed = true;
//             }
//         }
//
//         public bool CheckGate(User user, string gateName)
//         {
//             var gateNameBytes = Encoding.UTF8.GetBytes(gateName);
//             fixed (byte* gateNamePtr = gateNameBytes)
//             {
//                 return StatsigFFI.statsig_check_gate(_statsigRef, user.UserRef, gateNamePtr);
//             }
//         }
//
//         public Experiment GetExperiment(User user, string experimentName)
//         {
//             var experimentNameBytes = Encoding.UTF8.GetBytes(experimentName);
//             fixed (byte* experimentNamePtr = experimentNameBytes)
//             {
//                 var result = ReadStringFromPointer(
//                     StatsigFFI.statsig_get_experiment(_statsigRef, user.UserRef, experimentNamePtr)
//                 );
//                 return new Experiment(experimentName, "", new Dictionary<string, object>());
//             }
//         }
//
//         public string GetClientInitResponse(User user)
//         {
//             return ReadStringFromPointer(
//                 StatsigFFI.statsig_get_client_init_response(_statsigRef, user.UserRef)
//             );
//         }
//
//         private static string ReadStringFromPointer(byte* pointer)
//         {
//             if (pointer == null)
//             {
//                 return null;
//             }
//
//             var length = 0;
//             while (*(pointer + length) != 0)
//             {
//                 length++;
//             }
//
//             var responseBytes = new byte[length];
//             Marshal.Copy((IntPtr)pointer, responseBytes, 0, length);
//             return Encoding.UTF8.GetString(responseBytes);
//         }
//         
//         private StatsigServer(Ref* @ref)
//         {
//             _statsigRef = @ref;
//         }
//     }
// }