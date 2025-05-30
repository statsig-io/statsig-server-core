package com.statsig;

import java.util.concurrent.ExecutionException;
import org.junit.jupiter.api.Test;

public class StatsigTest {
  @Test
  public void testCreateWithKeyAndOptions() throws ExecutionException, InterruptedException {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setWaitForCountryLookupInit(true)
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
