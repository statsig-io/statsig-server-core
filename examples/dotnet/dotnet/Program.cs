using System.Diagnostics;
using StatsigServer;

// var statsig = await StatsigServer.Create("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW");
//
// var watch = Stopwatch.StartNew();
//
// var result = "";
// for (var i = 0; i < 1000; i++)
// {
//     var user = new User("user_" + i, "daniel@statsig.com");
//     var exp = statsig.GetExperiment(user, "running_exp_in_unlayered_with_holdout");
//     result = statsig.GetClientInitResponse(user);
// }
//
// watch.Stop();
//
// statsig.Dispose();

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
    var statsig = new Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW", options);
    await statsig.Initialize();

    var user = new StatsigUser("a-user", "daniel@statsig.com");
    var result = statsig.CheckGate(user, "test_public");
    Console.WriteLine("a_gate: " + result);
}

await Bar();

GC.Collect();



Console.WriteLine("Done");


