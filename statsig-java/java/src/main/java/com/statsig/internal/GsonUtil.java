package com.statsig.internal;

import com.google.gson.*;
import java.lang.reflect.Type;
import java.util.HashMap;
import java.util.Map;

public class GsonUtil {
  private static final Gson GSON =
      new GsonBuilder().registerTypeAdapter(Map.class, new MapDeserializer()).create();

  public static Gson getGson() {
    return GSON;
  }

  static class MapDeserializer implements JsonDeserializer<Map<String, JsonElement>> {

    @Override
    public Map<String, JsonElement> deserialize(
        JsonElement json, Type typeOfT, JsonDeserializationContext context) {
      Map<String, JsonElement> map = new HashMap<>();
      try {
        JsonObject obj = json.getAsJsonObject();

        for (Map.Entry<String, JsonElement> entry : obj.entrySet()) {
          map.put(entry.getKey(), entry.getValue());
        }
      } catch (Exception e) {
        e.printStackTrace();
      }
      return map;
    }
  }

  public static String getString(Map<String, JsonElement> value, String key, String defaultValue) {
    JsonElement res = value.get(key);
    return (res != null && res.isJsonPrimitive() && res.getAsJsonPrimitive().isString())
        ? res.getAsString()
        : defaultValue;
  }

  public static boolean getBoolean(
      Map<String, JsonElement> value, String key, boolean defaultValue) {
    JsonElement res = value.get(key);
    return (res != null && res.isJsonPrimitive() && res.getAsJsonPrimitive().isBoolean())
        ? res.getAsBoolean()
        : defaultValue;
  }

  public static double getDouble(Map<String, JsonElement> value, String key, double defaultValue) {
    JsonElement res = value.get(key);
    return (res != null && res.isJsonPrimitive() && res.getAsJsonPrimitive().isNumber())
        ? res.getAsDouble()
        : defaultValue;
  }

  public static int getInt(Map<String, JsonElement> value, String key, int defaultValue) {
    JsonElement res = value.get(key);
    return (res != null && res.isJsonPrimitive() && res.getAsJsonPrimitive().isNumber())
        ? res.getAsInt()
        : defaultValue;
  }

  public static long getLong(Map<String, JsonElement> value, String key, long defaultValue) {
    JsonElement res = value.get(key);
    return (res != null && res.isJsonPrimitive() && res.getAsJsonPrimitive().isNumber())
        ? res.getAsLong()
        : defaultValue;
  }

  public static Object[] getArray(
      Map<String, JsonElement> value, String key, Object[] defaultValue) {
    JsonElement res = value.get(key);
    if (res != null && res.isJsonArray()) {
      JsonArray jsonArray = res.getAsJsonArray();
      Object[] array = new Object[jsonArray.size()];
      for (int i = 0; i < jsonArray.size(); i++) {
        array[i] = jsonArray.get(i);
      }
      return array;
    }
    return defaultValue;
  }

  public static Map<String, Object> getMap(
      Map<String, JsonElement> value, String key, Map<String, Object> defaultValue) {
    JsonElement res = value.get(key);
    if (res != null && res.isJsonObject()) {
      JsonObject jsonObject = res.getAsJsonObject();
      Map<String, Object> map = new HashMap<>();
      for (Map.Entry<String, JsonElement> entry : jsonObject.entrySet()) {
        map.put(entry.getKey(), entry.getValue());
      }
      return map;
    }
    return defaultValue;
  }
}
