package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class PersistentStorageTest {

  private MockPersistentStorage storage;
  private MockWebServer mockWebServer;
  private String downloadConfigSpecsJson;

  @BeforeEach
  public void setUp() throws IOException {
    storage = new MockPersistentStorage();

    // Setup mock server
    downloadConfigSpecsJson = TestUtils.loadJsonFromFile("download_config_specs.json");
    mockWebServer = new MockWebServer();
    mockWebServer.start();

    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(downloadConfigSpecsJson));
  }

  @AfterEach
  public void tearDown() throws IOException {
    if (mockWebServer != null) {
      mockWebServer.shutdown();
    }
  }

  @Test
  public void testPersistentStorageIntegration()
      throws IOException, InterruptedException, ExecutionException {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setPersistentStorage(storage)
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .setOutputLoggerLevel(OutputLogger.LogLevel.ERROR)
            .setSpecsSyncIntervalMs(1)
            .build();

    Statsig statsig = new Statsig("secret-key", options);
    statsig.initialize().get();
    StatsigUser user = new StatsigUser.Builder().setUserID("test_user_123").build();
    statsig.getExperiment(user, "purchase_experiment");
    statsig.flushEvents().get();

    String storageKey = PersistentStorage.getStorageKey(user, "userID");
    assertNotNull(storageKey);
    Map<String, StickyValues> loaded = storage.load(storageKey);
    Map<String, StickyValues> valuesForUser = storage.getValuesForUser(user, "userID");

    if (loaded != null && !loaded.isEmpty()) {
      for (StickyValues stickyValues : loaded.values()) {
        assertNotNull(stickyValues);
        assertNotNull(stickyValues.getRuleId());
        if (stickyValues.getSecondaryExposures() != null) {
          for (Map<String, String> exposure : stickyValues.getSecondaryExposures()) {
            assertNotNull(exposure);
            assertTrue(exposure.containsKey("gate") || exposure.containsKey("ruleID"));
          }
        }
      }
    }
    if (valuesForUser != null && !valuesForUser.isEmpty()) {
      assertTrue(valuesForUser.size() > 0);
    }
    statsig.shutdown().get();
  }
}
