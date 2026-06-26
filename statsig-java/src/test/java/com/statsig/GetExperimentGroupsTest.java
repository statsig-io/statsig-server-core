package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class GetExperimentGroupsTest {
  private MockWebServer mockWebServer;
  private Statsig statsig;

  @BeforeEach
  public void setUp() throws IOException, InterruptedException, ExecutionException {
    String dcsContentJson =
        TestUtils.loadJsonFromFile("../statsig-rust/tests/data/eval_proj_dcs.json");

    mockWebServer = new MockWebServer();
    mockWebServer.start();

    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(dcsContentJson));

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .setLogEventUrl(mockWebServer.url("/v1/log_event").toString())
            .build();

    statsig = new Statsig("secret-key", options);
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
  public void testReturnsGroupsForKnownExperiment() {
    List<ExperimentGroup> groups = statsig.getExperimentGroups("test_experiment_no_targeting");

    Map<String, Map<String, Object>> groupsByName = new HashMap<>();
    for (ExperimentGroup group : groups) {
      groupsByName.put(group.getGroupName(), group.getReturnValue());
    }

    // Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assertEquals(3, groupsByName.size());
    assertTrue(groupsByName.containsKey("Control"));
    assertTrue(groupsByName.containsKey("Test"));
    assertTrue(groupsByName.containsKey("Test2"));
    assertEquals("control", groupsByName.get("Control").get("value"));
    assertEquals("test_1", groupsByName.get("Test").get("value"));
    assertEquals("test_2", groupsByName.get("Test2").get("value"));
  }

  @Test
  public void testReturnsEmptyListForUnknownExperiment() {
    List<ExperimentGroup> groups = statsig.getExperimentGroups("nonexistent_experiment");

    assertNotNull(groups);
    assertTrue(groups.isEmpty());
  }

  @Test
  public void testReturnsEmptyListForDynamicConfig() {
    List<ExperimentGroup> groups =
        statsig.getExperimentGroups("test_max_dynamic_config_size_again");

    assertNotNull(groups);
    assertTrue(groups.isEmpty());
  }

  @Test
  public void testReturnsEmptyListForInactiveExperiment() {
    List<ExperimentGroup> groups = statsig.getExperimentGroups("an_experiment1");

    assertNotNull(groups);
    assertTrue(groups.isEmpty());
  }
}
