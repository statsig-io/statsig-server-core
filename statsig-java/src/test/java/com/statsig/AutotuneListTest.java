package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.List;
import java.util.concurrent.ExecutionException;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class AutotuneListTest {
  private MockWebServer mockWebServer;
  private Statsig statsig;

  @BeforeEach
  public void setUp() throws IOException, InterruptedException, ExecutionException {
    String downloadConfigSpecsJson = TestUtils.loadJsonFromFile("download_config_specs.json");

    mockWebServer = new MockWebServer();
    mockWebServer.start();

    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(downloadConfigSpecsJson));

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .build();

    statsig = new Statsig("secret-test-key", options);
    statsig.initialize().get();
  }

  @AfterEach
  public void tearDown() throws IOException, ExecutionException, InterruptedException {
    if (statsig != null) {
      statsig.shutdown().get();
    }
    mockWebServer.shutdown();
  }

  @Test
  public void testGetAutotuneList() {
    List<String> autotuneList = statsig.getAutotuneList();

    assertNotNull(autotuneList);
    assertTrue(
        autotuneList.contains("test_autotune"),
        "Autotune list should contain 'test_autotune' from the DCS fixture");
  }
}
