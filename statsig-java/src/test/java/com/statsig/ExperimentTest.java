package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

public class ExperimentTest {

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

    List<Map<String, String>> exposures = experiment.getSecondaryExposures();
    assertEquals("global_holdout", exposures.get(0).get("gate"));
    assertEquals("false", exposures.get(0).get("gateValue"));
    assertEquals("3QoA4ncNdVGBaMt3N1KYjz:0.50:12", exposures.get(0).get("ruleID"));

    assertEquals("exp_holdout", exposures.get(1).get("gate"));
    assertEquals("false", exposures.get(1).get("gateValue"));
    assertEquals("1rEqLOpCROaRafv7ubGgax111", exposures.get(1).get("ruleID"));

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
