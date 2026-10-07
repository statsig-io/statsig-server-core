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
    assertNotNull(store.getParameterTypes());
    assertTrue(store.getParameterTypes().isEmpty());
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
  public void testGetParameterTypesForOverriddenParameters() {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);
    StatsigUser user = new StatsigUser.Builder().setUserID("123").build();

    Map<String, Object> values = new HashMap<>();
    values.put("bool_param", true);
    values.put("number_param", 123L);
    values.put("string_param", "a_string");
    values.put("object_param", new HashMap<String, Object>());
    values.put("array_param", new ArrayList<String>());
    statsigServer.overrideParameterStore("fake_store", values);

    Map<String, String> parameterTypes =
        statsigServer.getParameterStore(user, "fake_store").getParameterTypes();

    assertEquals("boolean", parameterTypes.get("bool_param"));
    assertEquals("number", parameterTypes.get("number_param"));
    assertEquals("string", parameterTypes.get("string_param"));
    assertEquals("object", parameterTypes.get("object_param"));
    assertEquals("array", parameterTypes.get("array_param"));
    assertEquals(
        Arrays.asList("array_param", "bool_param", "number_param", "object_param", "string_param"),
        new ArrayList<>(parameterTypes.keySet()));
  }

  @Test
  public void testGetParameterTypesForUserSpecificOverride() {
    StatsigOptions opt = new StatsigOptions.Builder().setDisableNetwork(true).build();
    Statsig statsigServer = new Statsig("test", opt);
    StatsigUser overriddenUser = new StatsigUser.Builder().setUserID("123").build();
    StatsigUser otherUser = new StatsigUser.Builder().setUserID("456").build();

    Map<String, Object> values = new HashMap<>();
    values.put("bool_param", true);
    values.put("number_param", 123L);
    statsigServer.overrideParameterStore("fake_store", values, "123");

    ParameterStore overriddenStore = statsigServer.getParameterStore(overriddenUser, "fake_store");
    assertEquals("boolean", overriddenStore.getParameterTypes().get("bool_param"));
    assertEquals("number", overriddenStore.getParameterTypes().get("number_param"));
    assertTrue(overriddenStore.getBoolean("bool_param", false));

    ParameterStore otherStore = statsigServer.getParameterStore(otherUser, "fake_store");
    assertTrue(otherStore.getParameterTypes().isEmpty());
    assertFalse(otherStore.getBoolean("bool_param", false));
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
