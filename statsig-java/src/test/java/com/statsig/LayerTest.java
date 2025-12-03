package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.alibaba.fastjson2.JSON;
import java.io.IOException;
import java.lang.reflect.Field;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

public class LayerTest {
  @Test
  public void testLayerDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("layer.json");
    Layer layer = JSON.parseObject(json, Layer.class);

    assertEquals("FJsjPaDrS4JydcV8A+6bAAH6PKUpax0Uh6WpfV1cltA=", layer.name);
    assertEquals("default", layer.ruleID);
    assertEquals("foo", layer.getString("a_param", "defaultVal"));
    assertEquals(45.6, layer.getDouble("another_param", 0.0));
    assertEquals("", layer.groupName);
    assertEquals("test_exper_name", layer.allocatedExperimentName);
  }

  @Test
  public void testNoExposureLoggingOnTypeMismatch() throws Exception {
    MockStatsig mockStatsig = new MockStatsig();

    Map<String, Object> value = new HashMap<>();
    value.put("int_param", "not_a_number");
    value.put("double_param", true);
    value.put("string_param", 123);
    value.put("boolean_param", "not_a_boolean");

    Layer layer = new Layer("test_layer", "rule_id", "group_name", value, "experiment_name", null);

    setupLayerWithMockStatsig(layer, mockStatsig);

    // Test getInt with String value (type mismatch)
    int intResult = layer.getInt("int_param", 999);
    assertEquals(999, intResult, "Should return fallback when type doesn't match");
    assertFalse(
        mockStatsig.wasExposureLogged("int_param"),
        "Should NOT log exposure when type doesn't match for getInt");

    // Test getDouble with Boolean value (type mismatch)
    double doubleResult = layer.getDouble("double_param", 123.45);
    assertEquals(123.45, doubleResult, "Should return fallback when type doesn't match");
    assertFalse(
        mockStatsig.wasExposureLogged("double_param"),
        "Should NOT log exposure when type doesn't match for getDouble");

    // Test getString with Number value (type mismatch)
    String stringResult = layer.getString("string_param", "fallback");
    assertEquals("fallback", stringResult, "Should return fallback when type doesn't match");
    assertFalse(
        mockStatsig.wasExposureLogged("string_param"),
        "Should NOT log exposure when type doesn't match for getString");

    // Test getBoolean with String value (type mismatch)
    boolean boolResult = layer.getBoolean("boolean_param", false);
    assertEquals(false, boolResult, "Should return fallback when type doesn't match");
    assertFalse(
        mockStatsig.wasExposureLogged("boolean_param"),
        "Should NOT log exposure when type doesn't match for getBoolean");
  }

  @Test
  public void testExposureLoggingOnTypeMatch() throws Exception {
    MockStatsig mockStatsig = new MockStatsig();

    Map<String, Object> value = new HashMap<>();
    value.put("int_param", 42);
    value.put("double_param", 3.14);
    value.put("string_param", "correct_string");
    value.put("boolean_param", true);

    Layer layer = new Layer("test_layer", "rule_id", "group_name", value, "experiment_name", null);

    setupLayerWithMockStatsig(layer, mockStatsig);

    int intResult = layer.getInt("int_param", 999);
    assertEquals(42, intResult);
    assertTrue(
        mockStatsig.wasExposureLogged("int_param"),
        "Should log exposure when type matches for getInt");

    double doubleResult = layer.getDouble("double_param", 0.0);
    assertEquals(3.14, doubleResult);
    assertTrue(
        mockStatsig.wasExposureLogged("double_param"),
        "Should log exposure when type matches for getDouble");

    String stringResult = layer.getString("string_param", "fallback");
    assertEquals("correct_string", stringResult);
    assertTrue(
        mockStatsig.wasExposureLogged("string_param"),
        "Should log exposure when type matches for getString");

    boolean boolResult = layer.getBoolean("boolean_param", false);
    assertEquals(true, boolResult);
    assertTrue(
        mockStatsig.wasExposureLogged("boolean_param"),
        "Should log exposure when type matches for getBoolean");
  }

  private static class MockStatsig extends Statsig {
    private final Map<String, Integer> exposureLogCount = new HashMap<>();

    public MockStatsig() {
      super("test-key-for-mock");
      try {
        Field refField = Statsig.class.getDeclaredField("ref");
        refField.setAccessible(true);
        refField.set(this, 0L);
      } catch (Exception e) {
      }
    }

    @Override
    void logLayerParamExposure(String layerJson, String param) {
      exposureLogCount.put(param, exposureLogCount.getOrDefault(param, 0) + 1);
    }

    public boolean wasExposureLogged(String param) {
      return exposureLogCount.containsKey(param) && exposureLogCount.get(param) > 0;
    }
  }

  private static void setupLayerWithMockStatsig(Layer layer, MockStatsig mockStatsig)
      throws Exception {
    Field statsigInstanceField = Layer.class.getDeclaredField("statsigInstance");
    statsigInstanceField.setAccessible(true);
    statsigInstanceField.set(layer, mockStatsig);

    Field rawJsonField = Layer.class.getDeclaredField("rawJson");
    rawJsonField.setAccessible(true);
    rawJsonField.set(layer, "{\"test\": \"json\"}");
  }
}
