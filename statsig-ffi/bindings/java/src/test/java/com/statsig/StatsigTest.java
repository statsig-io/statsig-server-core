package com.statsig;

import org.junit.jupiter.api.Test;

import java.util.concurrent.ExecutionException;

public class StatsigTest {
    @Test
    public void testCreateWithKeyAndOptions() throws ExecutionException, InterruptedException {
        StatsigOptions options = new StatsigOptions.Builder()
                .setEnableCountryLookup(true)
                .setEnvironment("staging")
                .build();

        Statsig statsig = new Statsig("secret-key", options);
        statsig.initialize().get();
    }

    @Test
    public void testCreateWithKey() throws ExecutionException, InterruptedException {
        Statsig statsig = new Statsig("secret-key");
        statsig.initialize().get();
    }
}
