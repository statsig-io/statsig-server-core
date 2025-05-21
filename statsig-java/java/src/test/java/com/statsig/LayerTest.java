package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.statsig.internal.JacksonUtil;
import java.io.IOException;
import org.junit.jupiter.api.Test;

public class LayerTest {

  private final ObjectMapper mapper = JacksonUtil.getObjectMapper();

  @Test
  public void testLayerDeserialization() throws IOException {
    String json = TestUtils.loadJsonFromFile("layer.json");
    Layer layer = JacksonUtil.fromJsonWithRawJson(json, Layer.class);

    assertEquals("FJsjPaDrS4JydcV8A+6bAAH6PKUpax0Uh6WpfV1cltA=", layer.name);
    assertEquals("default", layer.ruleID);
    assertEquals("foo", layer.getString("a_param", "defaultVal"));
    assertEquals(45.6, layer.getDouble("another_param", 0.0));
    assertEquals("", layer.groupName);
    assertEquals("test_exper_name", layer.allocatedExperimentName);
  }
}
