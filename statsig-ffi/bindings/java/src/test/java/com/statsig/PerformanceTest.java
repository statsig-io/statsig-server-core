package com.statsig;

import org.junit.jupiter.api.Test;

import java.util.concurrent.ExecutionException;

public class PerformanceTest {

    // no-op
    @Test
    public void test() throws ExecutionException, InterruptedException {
//        StatsigOptions options = new StatsigOptions.Builder().build();
//
//        Statsig statsig = new Statsig("secret-key", options);
//
//        statsig.initialize().get();
//        long startTime = System.nanoTime();
//        String result = null;
//
//        for (int i = 0; i < 1000; i++) {
//            String currUser = "user_" + i;
//            StatsigUser user = new StatsigUser.Builder()
//                    .setUserID(currUser)
//                    .setEmail("weihao@statsig.com")
//                    .build();
//            try {
//                result = statsig.getClientInitializeResponse(user, new ClientInitResponseOptions(HashAlgo.NONE));
//            } catch (Exception e) {
//                System.err.println("Error initializing client response for user " + currUser + ": " + e.getMessage());
//            }
//        }
//
//        long elapsedTime = System.nanoTime() - startTime;
//
//        if (result == null) {
//            System.out.println("Result is null");
//        } else if (result.isEmpty()) {
//            System.out.println("Result is empty");
//        } else {
//            System.out.println("Result: " + result);
//        }
//
//        System.out.println(elapsedTime / 1_000_000.0 + " ms"); // Convert nanoseconds to milliseconds
//        statsig.shutdown().get();
    }
}
