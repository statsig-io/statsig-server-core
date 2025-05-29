package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import okhttp3.mockwebserver.Dispatcher;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import okhttp3.mockwebserver.RecordedRequest;
import org.junit.jupiter.api.*;

public class IntializeAsyncTest {
  private MockWebServer mockWebServer;

  @BeforeEach
  public void setUp() throws IOException {
    mockWebServer = new MockWebServer();
    mockWebServer.start();
  }

  @AfterEach
  public void tearDown() throws IOException {
    mockWebServer.shutdown();
  }

  @Test
  public void testMultipleConcurrentInitializations() throws IOException, InterruptedException {
    final String dcsContentJson =
        TestUtils.loadJsonFromFile("../../statsig-rust/tests/data/eval_proj_dcs.json");

    Dispatcher dispatcher =
        new Dispatcher() {
          @Override
          public MockResponse dispatch(RecordedRequest request) {
            if (request.getPath().contains("/v2/download_config_specs")) {
              return new MockResponse()
                  .setResponseCode(200)
                  .setHeader("Content-Type", "application/json")
                  .setBody(dcsContentJson);
            }
            return new MockResponse().setResponseCode(404);
          }
        };

    mockWebServer.setDispatcher(dispatcher);

    final int numThreads = 5;
    final CountDownLatch startLatch = new CountDownLatch(1);
    final CountDownLatch completionLatch = new CountDownLatch(numThreads);
    final List<Long> startTimes = Collections.synchronizedList(new ArrayList<>());

    for (int i = 0; i < numThreads; i++) {
      final int threadId = i;
      new Thread(
              () -> {
                try {
                  startLatch.await();

                  StatsigOptions options =
                      new StatsigOptions.Builder()
                          .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
                          .setLogEventUrl(mockWebServer.url("/v1/log_event").toString())
                          .build();

                  Statsig statsig = new Statsig("secret-key-" + threadId, options);

                  startTimes.add(System.nanoTime());
                  statsig.initialize();
                } catch (Exception e) {
                  e.printStackTrace();
                } finally {
                  completionLatch.countDown();
                }
              })
          .start();
    }

    startLatch.countDown();
    completionLatch.await();

    // Evaluate if threads started at nearly the same time
    long min = Collections.min(startTimes);
    long max = Collections.max(startTimes);
    long diffMs = TimeUnit.NANOSECONDS.toMillis(max - min);
    assertTrue(diffMs < 20, "Threads were not started concurrently enough");
  }
}
