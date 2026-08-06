package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

public class ParameterStoreParameterListTest {

  @Test
  public void testGetParameterListForUnrecognizedStore() {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);
    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    ParameterStore store = statsigServer.getParameterStore(user, "not_a_real_store");

    assertNotNull(store.getParameterList());
    assertTrue(store.getParameterList().isEmpty());
  }

  @Test
  public void testGetParameterListReturnsSortedKeys() {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);
    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    Map<String, Object> values = new HashMap<>();
    values.put("zeta_param", "a_string");
    values.put("alpha_param", true);
    values.put("middle_param", 123L);
    values.put("nested_param", new HashMap<String, Object>());
    values.put("list_param", new ArrayList<String>());

    statsigServer.overrideParameterStore("fake_store", values);

    List<String> parameterList =
        statsigServer.getParameterStore(user, "fake_store").getParameterList();

    assertEquals(
        Arrays.asList("alpha_param", "list_param", "middle_param", "nested_param", "zeta_param"),
        parameterList);
  }

  @Test
  public void testGetParameterListReflectsOverriddenParameters() {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);
    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    assertTrue(statsigServer.getParameterStore(user, "fake_store").getParameterList().isEmpty());

    Map<String, Object> values = new HashMap<>();
    values.put("bool_param", true);
    statsigServer.overrideParameterStore("fake_store", values);

    assertEquals(
        Arrays.asList("bool_param"),
        statsigServer.getParameterStore(user, "fake_store").getParameterList());
  }
}
