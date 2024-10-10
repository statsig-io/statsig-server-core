using System.Diagnostics;
using Statsig;
using Statsig.Server;

var secret = Environment.GetEnvironmentVariable("test_api_key");
var statsig = await StatsigServer.Initialize(secret);

var watch = Stopwatch.StartNew();

object result = null;
for (var i = 0; i < 1000; i++)
{
    var user = new StatsigUser()
    {
        UserID = "user_" + i,
        Email = "daniel@statsig.com"
    };
    result = StatsigServer.GetClientInitializeResponse(user);
}

watch.Stop();

Console.WriteLine(result);
Console.WriteLine(watch.Elapsed.TotalMilliseconds);