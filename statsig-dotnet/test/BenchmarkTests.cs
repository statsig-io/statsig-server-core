using Xunit;
using System;
using System.Threading.Tasks;

namespace Statsig.tests
{

    public class BenchmarkTest
    {
        [Fact]
        public async Task PerfTest()
        {

            var options = new StatsigOptionsBuilder().Build();
            var statsigServer = new Statsig("secret-", options);
            var user = new StatsigUserBuilder()
                .SetUserID("admin")
                .SetEmail("fake@google.com")
                .Build();
            //var userNotPss = new StatsigUser("123", "weihao@gmail.com");
            var stopwatch = new System.Diagnostics.Stopwatch();

            await statsigServer.Initialize();
            stopwatch.Start();

            var iterations = 1000;
            for (int i = 0; i < iterations; i++)
            {
                // Console.WriteLine("Gate res: " + statsigServer.CheckGate(user, "test_string_comparisons"));
                // Console.WriteLine("Gate res: " + statsigServer.CheckGate(userNotPss, "test_string_comparisons"));
                statsigServer.GetClientInitializeResponse(user);
                if (i == iterations - 1)
                {
                    Console.WriteLine("GCIR: " + statsigServer.GetClientInitializeResponse(user));
                }
            }

            stopwatch.Stop();

            var elapsedTime = stopwatch.ElapsedMilliseconds;
            Console.WriteLine("Eval time (ms): " + elapsedTime);
        }
    }
}

