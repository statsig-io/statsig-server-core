package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.statsig.internal.JacksonUtil;
import java.io.IOException;
import org.junit.jupiter.api.Test;

public class FeatureGateTest {
  private static final ObjectMapper MAPPER = JacksonUtil.getObjectMapper();

  @Test
  public void testGateDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("gate.json");

    FeatureGate gate = JacksonUtil.fromJsonWithRawJson(json, FeatureGate.class);

    assertEquals("cPaGkTiRBuP1SfwoxRtTvhT6Vxpa4v/342Z1N0pXUlc=", gate.name);
    assertEquals("6X3qJgyfwA81IJ2dxI7lYp17", gate.ruleID);
    assertTrue(gate.value);
  }
}
