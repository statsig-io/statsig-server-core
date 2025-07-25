package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.*;
import org.junit.jupiter.api.*;

public class AsyncApiTest {
  Statsig statsig;

  @BeforeEach
  public void setup() {
    StatsigOptions options = new StatsigOptions.Builder().build();
    statsig = new Statsig("secret", options);
  }

  @Test
  public void testMultipleConcurrentInitializations() throws IOException, InterruptedException {
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

  @Test
  public void testInitializeWithDetailsAsyncOrder()
      throws InterruptedException, ExecutionException {

    List<String> state = new CopyOnWriteArrayList<>();
    CountDownLatch latch = new CountDownLatch(1);

    state.add("A");
    statsig
        .initializeWithDetails()
        .thenAccept(
            details -> {
              assertNotNull(details);
              state.add("C");
              latch.countDown();
            });
    state.add("B");

    assertTrue(latch.await(5, TimeUnit.SECONDS), "initializeWithDetails() should complete");
    assertEquals(Arrays.asList("A", "B", "C"), state, "Expected order A, B, C");
  }

  @Test
  public void testFlushEventsAsyncOrder() throws InterruptedException, ExecutionException {
    statsig.initialize().get();

    List<String> state = new CopyOnWriteArrayList<>();
    CountDownLatch latch = new CountDownLatch(1);

    state.add("A");
    CompletableFuture<Void> initFuture = statsig.flushEvents();
    initFuture.thenRun(
        () -> {
          state.add("C");
          latch.countDown();
        });
    state.add("B");

    assertTrue(latch.await(5, TimeUnit.SECONDS), "flushEvents() should complete");
    assertEquals(Arrays.asList("A", "B", "C"), state, "Expected order A, B, C");
  }

  @Test
  public void testShutdownAsyncOrder() throws InterruptedException, ExecutionException {
    statsig.initialize().get();

    List<String> state = new CopyOnWriteArrayList<>();
    CountDownLatch latch = new CountDownLatch(1);

    state.add("A");
    statsig
        .shutdown()
        .thenRun(
            () -> {
              state.add("C");
              latch.countDown();
            });
    state.add("B");

    assertTrue(latch.await(5, TimeUnit.SECONDS), "shutdown() should complete");
    assertTrue(state.indexOf("A") < state.indexOf("B"));
    assertTrue(state.indexOf("B") < state.indexOf("C"));
  }
}
