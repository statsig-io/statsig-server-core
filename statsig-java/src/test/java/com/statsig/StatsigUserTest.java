package com.statsig;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.statsig.internal.JacksonUtil;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class StatsigUserTest {
  private ObjectMapper mapper;
  private StatsigUser user;

  @BeforeEach
  public void setUp() {
    mapper = JacksonUtil.getObjectMapper();

    Map<String, String> customIDs = new HashMap<>();
    customIDs.put("ID1", "custom1");
    customIDs.put("ID2", "custom2");

    Map<String, Object> custom = new HashMap<>();
    custom.put("age", 25);
    custom.put("is_premium", true);

    Map<String, String> privateAttributes = new HashMap<>();
    privateAttributes.put("secret_key", "secretValue");

    user =
        new StatsigUser.Builder()
            .setUserID("user123")
            .setCustomIDs(customIDs)
            .setEmail("test@example.com")
            .setIp("192.168.1.1")
            .setLocale("en-US")
            .setAppVersion("1.0.0")
            .setUserAgent("Mozilla/5.0")
            .setCountry("US")
            .setPrivateAttributes(privateAttributes)
            .setCustom(custom)
            .build();
  }

  @Test
  public void testMemoryUsage() {
    Runtime runtime = Runtime.getRuntime();

    long totalMemory = runtime.totalMemory(); // Current heap allocated
    long freeMemory = runtime.freeMemory(); // Free heap in allocated memory
    long usedMemoryPrev = totalMemory - freeMemory; // Used memory
    for (int i = 0; i < 1000; i++) {
      StatsigUser user = new StatsigUser.Builder().setUserID("a").build();
    }
    System.gc();
    long totalMemoryAfter = runtime.totalMemory(); // Current heap allocated
    long freeMemoryAfter = runtime.freeMemory(); // Free heap in allocated memory
    long usedMemoryAfter = totalMemoryAfter - freeMemoryAfter; // Used memory
    assert ((usedMemoryAfter - usedMemoryPrev) < 10); // Assert no memory leak
  }
}
