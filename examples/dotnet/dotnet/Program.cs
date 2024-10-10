using System.Diagnostics;
using StatsigServer;



void Foo()
{
    var options = new StatsigOptions();

   
    Task task1 = Task.Run(() => options.Dispose());
    Task task2 = Task.Run(() => options.Dispose());
    Task task3 = Task.Run(() => options.Dispose());

}

Foo();

async Task Bar()
{
    var options = new StatsigOptions();
    var sdkKey = Environment.GetEnvironmentVariable("test_api_key");
    var statsig = new Statsig(sdkKey, options);
    await statsig.Initialize();

    var user = new StatsigUser("a-user", "daniel@statsig.com");
    var result = statsig.CheckGate(user, "test_public");
    Console.WriteLine("a_gate: " + result);
}

await Bar();

GC.Collect();



Console.WriteLine("Done");


