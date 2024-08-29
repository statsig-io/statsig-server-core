package example.statsig;

import com.google.gson.Gson;
import com.statsig.Statsig;
import com.statsig.StatsigJNI;
import com.statsig.StatsigOptions;
import com.statsig.StatsigUser;

import java.util.*;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutionException;


public class Main {
    public static void main(String[] args) throws ExecutionException, InterruptedException {
        if (!StatsigJNI.isLibraryLoaded()) {
            System.out.println("Statsig library not loaded");
            return;
        }

        StatsigOptions options = new StatsigOptions.Builder().build();
        Statsig statsig = new Statsig("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW", options);

        statsig.initialize().get();

        StatsigUser user = new StatsigUser("a_user", "user@statsig.com");

        boolean check = statsig.checkGate(user, "test_public");

        System.out.println("test_public: " + check);

        statsig.flushEvents().get();
    }
}