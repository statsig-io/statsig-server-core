package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.alibaba.fastjson2.JSONObject;
import java.io.IOException;
import org.junit.jupiter.api.Test;

public class FeatureGateTest {
  @Test
  public void testGateDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("gate.json");

    FeatureGate gate = JSONObject.parseObject(json, FeatureGate.class);

    assertEquals("cPaGkTiRBuP1SfwoxRtTvhT6Vxpa4v/342Z1N0pXUlc=", gate.name);
    assertEquals("6X3qJgyfwA81IJ2dxI7lYp17", gate.ruleID);
    assertTrue(gate.value);
  }
}
