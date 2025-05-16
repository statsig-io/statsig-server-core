package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;
import java.io.IOException;
import org.junit.jupiter.api.Test;

public class ExperimentTest {

  private final Gson gson = GsonUtil.getGson();

  @Test
  public void testDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("experiment.json");

    Experiment experiment = Experiment.fromJson(json);

    // basic
    assertNotNull(experiment);
    assertEquals("Test Experiment", experiment.name);
    assertEquals("rule-123", experiment.ruleID);
    assertEquals("test-group", experiment.groupName);

    // 2nd exposures
    assertNotNull(experiment.getSecondaryExposures());
    assertEquals(2, experiment.getSecondaryExposures().size());
    assertEquals(
        "[{gate=global_holdout, gateValue=false, ruleID=3QoA4ncNdVGBaMt3N1KYjz:0.50:12}, "
            + "{gate=exp_holdout, gateValue=false, ruleID=1rEqLOpCROaRafv7ubGgax111}]",
        experiment.getSecondaryExposures().toString());

    // raw json
    assertEquals(json, experiment.getRawJson());

    // eval details
    assertEquals("Network:Recognized", experiment.getEvaluationDetails().reason);
    assertEquals(1740188822154L, experiment.getEvaluationDetails().lcut);
    assertEquals(1741990144165L, experiment.getEvaluationDetails().receivedAt);

    String expectedStringValue = "value1";
    int expectedIntValue = 2;

    assertEquals(expectedStringValue, experiment.getString("key1", "defaultValue"));
    assertEquals(expectedIntValue, experiment.getInt("key2", -1));
  }
}
