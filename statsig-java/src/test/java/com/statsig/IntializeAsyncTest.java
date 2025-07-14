package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.*;

public class IntializeAsyncTest {

  @Test
  public void testMultipleConcurrentInitializations() throws IOException, InterruptedException {
    StatsigOptions options = new StatsigOptions.Builder().build();
    Statsig statsig = new Statsig("secret", options);

    List<String> state = new CopyOnWriteArrayList<>();
    CountDownLatch latch = new CountDownLatch(1);

    state.add("A");
    CompletableFuture<Void> initFuture = statsig.initialize();
    initFuture.thenRun(
        () -> {
          state.add("C");
          latch.countDown();
        });
    state.add("B");

    boolean completed = latch.await(5, TimeUnit.SECONDS);
    assertTrue(completed, "Callback should have completed within 5 seconds");
    assertEquals(Arrays.asList("A", "B", "C"), state, "Expected order A, B, C");
  }
}
