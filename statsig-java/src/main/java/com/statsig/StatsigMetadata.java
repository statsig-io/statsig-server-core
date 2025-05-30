package com.statsig;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.statsig.internal.JacksonUtil;
import java.util.HashMap;
import java.util.Map;

public class StatsigMetadata {
  public static String getSerializedCopy() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("os", System.getProperty("os.name"));
    metadata.put("arch", System.getProperty("os.arch"));
    metadata.put("languageVersion", System.getProperty("java.version"));

    try {
      return JacksonUtil.getObjectMapper().writeValueAsString(metadata);
    } catch (JsonProcessingException e) {
      return "{}";
    }
  }
}
