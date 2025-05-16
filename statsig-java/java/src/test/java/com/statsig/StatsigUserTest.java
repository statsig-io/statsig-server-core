package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import com.google.gson.Gson;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class StatsigUserTest {
  private Gson gson;
  private StatsigUser user;

  @BeforeEach
  public void setUp() {
    gson = new Gson();

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
  public void testSerialization() {
    // Serialize the StatsigUser object to JSON
    String json = gson.toJson(user);
    assertNotNull(json);

    assert (json.contains("\"userID\":\"user123\""));
    assert (json.contains("\"email\":\"test@example.com\""));
    assert (json.contains("\"age\":25"));
    assert (json.contains("\"is_premium\":true"));
  }

  @Test
  public void testDeserialization() {
    String json =
        "{"
            + "\"userID\":\"user123\","
            + "\"customIDs\":{\"ID1\":\"custom1\",\"ID2\":\"custom2\"},"
            + "\"email\":\"test@example.com\","
            + "\"ip\":\"192.168.1.1\","
            + "\"locale\":\"en-US\","
            + "\"appVersion\":\"1.0.0\","
            + "\"userAgent\":\"Mozilla/5.0\","
            + "\"country\":\"US\","
            + "\"privateAttributes\":{\"secret_key\":\"secretValue\"},"
            + "\"custom\":{\"age\":25,\"is_premium\":true}"
            + "}";

    StatsigUser deserializedUser = gson.fromJson(json, StatsigUser.class);

    assertEquals("user123", deserializedUser.getUserID());
    assertEquals("test@example.com", deserializedUser.getEmail());
    assertEquals("192.168.1.1", deserializedUser.getIp());
    assertEquals("US", deserializedUser.getCountry());
    assertEquals("en-US", deserializedUser.getLocale());
    assertEquals("1.0.0", deserializedUser.getAppVersion());
    assertEquals(2, deserializedUser.getCustom().size());
    assertEquals(25.0, deserializedUser.getCustom().get("age"));
    assertEquals(true, deserializedUser.getCustom().get("is_premium"));

    assertEquals("secretValue", deserializedUser.getPrivateAttributes().get("secret_key"));
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
