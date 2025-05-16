package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.*;

public class StatsigInitializeWithDetails {

  private MockWebServer mockWebServer;
  private Statsig statsig;

  @BeforeEach
  public void setUp() throws IOException, InterruptedException, ExecutionException {

    String dcs_content_json =
        TestUtils.loadJsonFromFile("../../statsig-rust/tests/data/eval_proj_dcs.json");

    mockWebServer = new MockWebServer();
    mockWebServer.start();

    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(dcs_content_json));
  }

  @AfterEach
  public void tearDown() throws IOException, ExecutionException, InterruptedException {
    if (statsig != null) {
      statsig.shutdown().get();
    }
    mockWebServer.shutdown();
  }

  @Test
  public void testInitializeWithDetailsSuccess() throws InterruptedException, ExecutionException {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .setLogEventUrl(mockWebServer.url("/v1/log_event").toString())
            .build();

    Statsig statsig = new Statsig("secret-key", options);

    CompletableFuture<InitializeDetails> future = statsig.initializeWithDetails();
    InitializeDetails init_details = future.get();
    assertTrue(init_details.getDuration() > 0);
    assertTrue(init_details.getIsInitSuccess());
    assertTrue(init_details.getIsConfigSpecReady());
    assertFalse(init_details.getIsIdListReady());
    assertEquals(init_details.getSource(), "Network");
    assertNull(init_details.getFailureDetails());
  }

  @Test
  public void testInitializeWithDetailsFailure() throws InterruptedException, ExecutionException {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("http://invalid.url").toString())
            .setLogEventUrl(mockWebServer.url("/v1/log_event").toString())
            .build();

    Statsig statsig = new Statsig("secret-key", options);

    CompletableFuture<InitializeDetails> future = statsig.initializeWithDetails();
    InitializeDetails init_details = future.get();

    assertTrue(init_details.getDuration() >= 0);
    assertTrue(init_details.getIsInitSuccess());
    assertFalse(init_details.getIsConfigSpecReady());
    assertFalse(init_details.getIsIdListReady());
    assertEquals(init_details.getSource(), "NoValues");
    assertNotEquals(init_details.getFailureDetails(), null);
  }
}
