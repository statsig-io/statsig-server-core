package com.statsig;

import org.junit.jupiter.api.Test;

import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ExecutionException;

import static org.junit.jupiter.api.Assertions.fail;

public class PerformanceTest {
    @Test
    public void test() throws ExecutionException, InterruptedException {
        StatsigOptions options = new StatsigOptions.Builder().build();

        Statsig statsig = new Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW", options);

        statsig.initialize().get();
        long startTime = System.nanoTime();
        String result = null;

        for (int i = 0; i < 1000; i++) {
            String currUser = "user_" + i;
            try (StatsigUser user = new StatsigUser(currUser, "weihao@statsig.com")) {
                try {
                    result = statsig.getClientInitializeResponse(user);
                } catch (Exception e) {
                    System.err.println("Error initializing client response for user " + currUser + ": " + e.getMessage());
                }
            } catch (Exception e) {
                System.err.println("Error creating StatsigUser for user " + currUser + ": " + e.getMessage());
            }
        }

        long elapsedTime = System.nanoTime() - startTime;

        if (result == null) {
            System.out.println("Result is null");
        } else if (result.isEmpty()) {
            System.out.println("Result is empty");
        } else {
            System.out.println("Result: " + result);
        }

        System.out.println(elapsedTime / 1_000_000.0 + " ms"); // Convert nanoseconds to milliseconds

        statsig.close();
        options.close();
    }
}
