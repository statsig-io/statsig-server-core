package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;
import java.io.IOException;
import org.junit.jupiter.api.Test;

public class FeatureGateTest {
  private static final Gson GSON = GsonUtil.getGson();

  @Test
  public void testGateDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("gate.json");

    FeatureGate gate = GSON.fromJson(json, FeatureGate.class);

    assertEquals("cPaGkTiRBuP1SfwoxRtTvhT6Vxpa4v/342Z1N0pXUlc=", gate.name);
    assertEquals("6X3qJgyfwA81IJ2dxI7lYp17", gate.ruleID);
    assertTrue(gate.value);
  }
}
