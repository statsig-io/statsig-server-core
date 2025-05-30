package com.statsig;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.statsig.internal.JacksonUtil;
import java.util.Map;

public class StatsigMetadata {
  public static String getSerializedCopy() {
    Map<String, String> metadata =
        Map.of(
            "os", System.getProperty("os.name"),
            "arch", System.getProperty("os.arch"),
            "languageVersion", System.getProperty("java.version"));

    try {
      return JacksonUtil.getObjectMapper().writeValueAsString(metadata);
    } catch (JsonProcessingException e) {
      return "{}";
    }
  }
}
