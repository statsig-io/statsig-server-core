package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * Covers the enforceOverrides / enforceTargeting persistent-assignment options. Fixture
 * (enforce_sticky_dcs.json): experiment `enforce_exp` with a console override rule matching userID
 * `override-user`, a targeting gate passing only users with custom `targeted=yes`, and layer
 * `enforce_layer` delegating to the experiment.
 */
public class EnforceStickyValuesTest {

  private MockWebServer mockWebServer;
  private Statsig statsig;

  @BeforeEach
  public void setUp() throws IOException, ExecutionException, InterruptedException {
    String dcsJson = TestUtils.loadJsonFromFile("enforce_sticky_dcs.json");
    mockWebServer = new MockWebServer();
    mockWebServer.start();
    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody(dcsJson));

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .setOutputLoggerLevel(OutputLogger.LogLevel.ERROR)
            // userPersistedValues are only honored when a persistent storage
            // adapter is configured.
            .setPersistentStorage(new MockPersistentStorage())
            .build();
    statsig = new Statsig("secret-key", options);
    statsig.initialize().get();
  }

  @AfterEach
  public void tearDown() throws IOException, ExecutionException, InterruptedException {
    if (statsig != null) {
      statsig.shutdown().get();
    }
    if (mockWebServer != null) {
      mockWebServer.shutdown();
    }
  }

  private static StatsigUser makeUser(String userID, boolean targeted) {
    Map<String, Object> custom = new HashMap<>();
    custom.put("targeted", targeted ? "yes" : "no");
    return new StatsigUser.Builder().setUserID(userID).setCustom(custom).build();
  }

  private static Map<String, StickyValues> stickyValues(String configName, String configDelegate) {
    Map<String, Object> jsonValue = new HashMap<>();
    jsonValue.put("value", "sticky_value");
    StickyValues sticky =
        new StickyValues(
            true,
            jsonValue,
            "sticky_rule_id",
            "Sticky Group",
            Collections.emptyList(),
            Collections.emptyList(),
            configDelegate,
            null,
            1700000000000L,
            null);
    Map<String, StickyValues> values = new HashMap<>();
    values.put(configName, sticky);
    return values;
  }

  // ---------------------------------------------------------------- experiments

  @Test
  public void stickyValueWinsWithoutEnforceOverrides() {
    StatsigUser user = makeUser("override-user", true);
    GetExperimentOptions options =
        new GetExperimentOptions(false, stickyValues("enforce_exp", null), false, false);

    Experiment experiment = statsig.getExperiment(user, "enforce_exp", options);

    assertEquals("sticky_value", experiment.getValue().get("value"));
    assertEquals("sticky_rule_id", experiment.getRuleID());
  }

  @Test
  public void enforceOverridesLetsOverrideWinOverSticky() {
    StatsigUser user = makeUser("override-user", true);
    GetExperimentOptions options =
        new GetExperimentOptions(false, stickyValues("enforce_exp", null), true, false);

    Experiment experiment = statsig.getExperiment(user, "enforce_exp", options);

    assertEquals("override_value", experiment.getValue().get("value"));
    assertEquals("override_rule:userID:id_override", experiment.getRuleID());
  }

  @Test
  public void enforceOverridesKeepsStickyWhenNoOverrideMatches() {
    StatsigUser user = makeUser("plain-user", true);
    GetExperimentOptions options =
        new GetExperimentOptions(false, stickyValues("enforce_exp", null), true, false);

    Experiment experiment = statsig.getExperiment(user, "enforce_exp", options);

    assertEquals("sticky_value", experiment.getValue().get("value"));
    assertEquals("sticky_rule_id", experiment.getRuleID());
  }

  @Test
  public void enforceTargetingKeepsStickyWhenStillTargeted() {
    StatsigUser user = makeUser("plain-user", true);
    GetExperimentOptions options =
        new GetExperimentOptions(false, stickyValues("enforce_exp", null), false, true);

    Experiment experiment = statsig.getExperiment(user, "enforce_exp", options);

    assertEquals("sticky_value", experiment.getValue().get("value"));
  }

  @Test
  public void enforceTargetingDropsStickyWhenNoLongerTargeted() {
    StatsigUser user = makeUser("plain-user", false);
    GetExperimentOptions options =
        new GetExperimentOptions(false, stickyValues("enforce_exp", null), false, true);

    Experiment experiment = statsig.getExperiment(user, "enforce_exp", options);

    assertNotEquals("sticky_value", experiment.getValue().get("value"));
    assertEquals("targetingGate", experiment.getRuleID());
  }

  // ---------------------------------------------------------------- layers

  @Test
  public void layerStickyValueWinsWithoutEnforceOverrides() {
    StatsigUser user = makeUser("override-user", true);
    GetLayerOptions options =
        new GetLayerOptions(false, stickyValues("enforce_layer", "enforce_exp"), false, false);

    Layer layer = statsig.getLayer(user, "enforce_layer", options);

    assertEquals("sticky_value", layer.getString("value", "fallback"));
  }

  @Test
  public void layerEnforceOverridesLetsOverrideWinOverSticky() {
    StatsigUser user = makeUser("override-user", true);
    GetLayerOptions options =
        new GetLayerOptions(false, stickyValues("enforce_layer", "enforce_exp"), true, false);

    Layer layer = statsig.getLayer(user, "enforce_layer", options);

    assertEquals("override_value", layer.getString("value", "fallback"));
  }

  @Test
  public void layerEnforceTargetingDropsStickyWhenNoLongerTargeted() {
    StatsigUser user = makeUser("plain-user", false);
    GetLayerOptions options =
        new GetLayerOptions(false, stickyValues("enforce_layer", "enforce_exp"), false, true);

    Layer layer = statsig.getLayer(user, "enforce_layer", options);

    assertNotEquals("sticky_value", layer.getString("value", "fallback"));
  }
}
