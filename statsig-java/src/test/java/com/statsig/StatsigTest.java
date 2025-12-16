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

  @Test
  public void testCreateWithNullKey() {
    System.out.println("[TEST] Testing Statsig creation with null SDK key");
    try {
      Statsig statsig = new Statsig((String) null);
      System.out.println("[TEST] Statsig instance created, ref: " + statsig.getRef());
      statsig.initialize().get();
      // If we get here, the instance was created but should be invalid
      // Try to use it to see what happens
      boolean result =
          statsig.checkGate(new StatsigUser.Builder().setUserID("user-123").build(), "test-gate");
      System.out.println("[TEST] Gate check result: " + result);
    } catch (Exception e) {
      System.out.println("[TEST] Exception caught: " + e.getClass().getName());
      System.out.println("[TEST] Exception message: " + e.getMessage());
      e.printStackTrace();
    }
  }
}
