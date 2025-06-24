package com.statsig;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class DataStoreTest {

  private static final String RULESET_KEY = "/v2/download_config_specs";
  private MockDataStore dataStore;
  private MockWebServer mockWebServer;
  private String downloadConfigSpecsJson;

  @BeforeEach
  public void setup() throws IOException {
    dataStore = new MockDataStore();
    downloadConfigSpecsJson = TestUtils.loadJsonFromFile("download_config_specs.json");

    mockWebServer = new MockWebServer();
    mockWebServer.start(8899);

    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(downloadConfigSpecsJson));
  }

  @AfterEach
  public void tearDown() throws IOException {
    mockWebServer.shutdown();
  }

  @Test
  public void testDataStoreGetSet() throws Exception {
    dataStore.shouldPoll = true;

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setDataStore(dataStore)
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .build();

    Statsig statsig = new Statsig("secret-key", options);
    statsig.initialize().get();

    statsig.shutdown().get();

    Assertions.assertTrue(dataStore.initCalled);
    Assertions.assertTrue(dataStore.shutdownCalled);
    Assertions.assertEquals(dataStore.contentSet, downloadConfigSpecsJson);
  }

  static class MockDataStore implements DataStore {
    public boolean initCalled = false;
    public boolean shutdownCalled = false;
    public String contentSet = null;
    public boolean shouldPoll = false;

    public Map<String, DataStoreResponse> store = new HashMap<>();
    public DataStoreResponse nextSetResponse = null;

    @Override
    public CompletableFuture<Void> initialize() {
      this.initCalled = true;
      return CompletableFuture.completedFuture(null);
    }

    @Override
    public CompletableFuture<Void> shutdown() {
      this.shutdownCalled = true;
      return CompletableFuture.completedFuture(null);
    }

    @Override
    public CompletableFuture<DataStoreResponse> get(String key) {
      return CompletableFuture.completedFuture(
          store.getOrDefault(
              key, nextSetResponse != null ? nextSetResponse : new DataStoreResponse(null, null)));
    }

    @Override
    public CompletableFuture<Void> set(String key, String value, Long time) {
      contentSet = value;
      store.put(key, new DataStoreResponse(value, time != null ? time : 0L));
      return CompletableFuture.completedFuture(null);
    }

    @Override
    public CompletableFuture<Boolean> supportPollingUpdatesFor(String key) {
      return CompletableFuture.completedFuture(shouldPoll);
    }
  }
}
