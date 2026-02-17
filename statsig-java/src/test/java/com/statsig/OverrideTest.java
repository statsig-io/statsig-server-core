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
}
