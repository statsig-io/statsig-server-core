package example.statsig;

import com.statsig.OutputLogger;
import com.statsig.Statsig;
import com.statsig.StatsigOptions;
import com.statsig.StatsigUser;

import java.util.concurrent.ExecutionException;


public class Main {
    public static void main(String[] args) throws ExecutionException, InterruptedException {
        StatsigOptions options = new StatsigOptions.Builder().setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG).build();
        Statsig statsig = new Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW", options);

        statsig.initialize().get();

        StatsigUser user = new StatsigUser("a_user", "user@statsig.com");

        boolean check = statsig.checkGate(user, "test_public");

        System.out.println("test_public: " + check);

        statsig.flushEvents().get();
    }
}