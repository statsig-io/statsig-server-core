package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

public class OverrideTest {
  @Test
  public void testOverrideDynamicConfig() throws IOException {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);

    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    DynamicConfig c1 = statsigServer.getDynamicConfig(user, "fake_config");
    assertTrue(c1.value == null || c1.value.isEmpty());

    Map<String, Object> values = new HashMap<>();
    List<String> list = new ArrayList<>();
    list.add("123");
    list.add("456");
    values.put("key", list);
    statsigServer.overrideDynamicConfig("fake_config", values);
    DynamicConfig c2 = statsigServer.getDynamicConfig(user, "fake_config");
    Object[] temp = c2.getArray("key", new String[0]);

    for (int i = 0; i < temp.length; i++) {
      assertEquals(list.get(i), temp[i].toString());
    }
  }

  @Test
  public void testOverrideParameterStore() throws IOException {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);

    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    ParameterStore c1 = statsigServer.getParameterStore(user, "fake_store");
    assertTrue(c1.getBoolean("bool", false) == false);

    Map<String, Object> values = new HashMap<>();
    List<String> list = new ArrayList<>();
    list.add("123");
    list.add("456");
    values.put("key", list);

    values.put("long", 123L);
    values.put("double", 123.0);
    values.put("int", 123);
    values.put("bool", true);
    values.put("val", "string");

    Map<String, Object> map = new HashMap<>();
    map.put("key1", "val1");
    map.put("key2", "val2");

    values.put("map", map);

    statsigServer.overrideParameterStore("fake_store", values);
    ParameterStore c2 = statsigServer.getParameterStore(user, "fake_store");
    Object[] temp = c2.getArray("key", new String[0]);

    Map<String, Object> fallbackMap = new HashMap<>();

    Map<String, Object> mapVal = c2.getMap("map", fallbackMap);

    boolean boolVal = c2.getBoolean("bool", false);

    String strVal = c2.getString("val", "default");

    long longVal = c2.getLong("long", 0L);

    double doubleVal = c2.getDouble("double", 0.0);

    int intVal = c2.getInt("int", 0);

    for (int i = 0; i < temp.length; i++) {
      assertEquals(list.get(i), temp[i].toString());
    }

    assertEquals("val1", mapVal.getOrDefault("key1", "default"));
    assertEquals("val2", mapVal.getOrDefault("key2", "default"));

    assertEquals(true, boolVal);
    assertEquals("string", strVal);
    assertEquals(123L, longVal);
    assertEquals(123.0, doubleVal);
    assertEquals(123, intVal);
  }
}
