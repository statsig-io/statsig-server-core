package com.statsig;

import static java.util.stream.Collectors.toList;
import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.Arrays;
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
    ExperimentGroupsResult result = statsig.getExperimentGroups("test_experiment_no_targeting");

    assertEquals(Boolean.TRUE, result.getIsExperimentActive());

    Map<String, ExperimentGroup> groupsByName = new HashMap<>();
    for (ExperimentGroup group : result.getGroups()) {
      groupsByName.put(group.getGroupName(), group);
    }

    // Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assertEquals(3, groupsByName.size());
    assertTrue(groupsByName.containsKey("Control"));
    assertTrue(groupsByName.containsKey("Test"));
    assertTrue(groupsByName.containsKey("Test2"));
    assertEquals("control", groupsByName.get("Control").getReturnValue().get("value"));
    assertEquals("54QJztEPRLXK7ZCvXeY9q4", groupsByName.get("Control").getRuleID());
    assertEquals("userID", groupsByName.get("Control").getIDType());
    assertEquals("test_1", groupsByName.get("Test").getReturnValue().get("value"));
    assertEquals("test_2", groupsByName.get("Test2").getReturnValue().get("value"));
  }

  @Test
  public void testReturnsNullActiveStateForUnknownExperiment() {
    ExperimentGroupsResult result = statsig.getExperimentGroups("nonexistent_experiment");

    assertNull(result.getIsExperimentActive());
    assertTrue(result.getGroups().isEmpty());
  }

  @Test
  public void testReturnsNullActiveStateForDynamicConfig() {
    ExperimentGroupsResult result =
        statsig.getExperimentGroups("test_max_dynamic_config_size_again");

    assertNull(result.getIsExperimentActive());
    assertTrue(result.getGroups().isEmpty());
  }

  @Test
  public void testReturnsGroupsForInactiveExperiment() {
    // test_switchback has isActive: false; groups are still returned along with the flag.
    ExperimentGroupsResult result = statsig.getExperimentGroups("test_switchback");

    assertEquals(Boolean.FALSE, result.getIsExperimentActive());

    // Only the experiment group rules are returned (non-group rules are excluded).
    List<String> groupNames =
        result.getGroups().stream().map(ExperimentGroup::getGroupName).sorted().collect(toList());
    assertEquals(Arrays.asList("Control", "Test"), groupNames);
  }
}
