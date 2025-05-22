package com.statsig.internal;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonDeserializer;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.module.SimpleModule;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

public class JacksonUtil {
  private static final ObjectMapper MAPPER;

  static {
    MAPPER = new ObjectMapper();
    MAPPER.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    MAPPER.configure(DeserializationFeature.ACCEPT_SINGLE_VALUE_AS_ARRAY, true);

    SimpleModule module = new SimpleModule();
    module.addDeserializer(Map.class, new MapDeserializer());
    MAPPER.registerModule(module);
  }

  public static ObjectMapper getObjectMapper() {
    return MAPPER;
  }

  /**
   * Helper method to get a String value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     String
   * @return the String value or the default value
   */
  public static String getString(Map<String, Object> map, String key, String defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof String) {
      return (String) value;
    }
    return defaultValue;
  }

  /**
   * Helper method to get a boolean value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     boolean
   * @return the boolean value or the default value
   */
  public static boolean getBoolean(Map<String, Object> map, String key, boolean defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Boolean) {
      return (Boolean) value;
    }
    return defaultValue;
  }

  /**
   * Helper method to get a double value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     number
   * @return the double value or the default value
   */
  public static double getDouble(Map<String, Object> map, String key, double defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Number) {
      return ((Number) value).doubleValue();
    }
    return defaultValue;
  }

  /**
   * Helper method to get an int value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     number
   * @return the int value or the default value
   */
  public static int getInt(Map<String, Object> map, String key, int defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Number) {
      return ((Number) value).intValue();
    }
    return defaultValue;
  }

  /**
   * Helper method to get a long value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     number
   * @return the long value or the default value
   */
  public static long getLong(Map<String, Object> map, String key, long defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Number) {
      return ((Number) value).longValue();
    }
    return defaultValue;
  }

  /**
   * Helper method to get an array value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not an
   *     array
   * @return the array value or the default value
   */
  public static Object[] getArray(Map<String, Object> map, String key, Object[] defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Object[]) {
      return (Object[]) value;
    }
    return defaultValue;
  }

  /**
   * Helper method to get a Map value from a Map.
   *
   * @param map the map to get the value from
   * @param key the key to get the value for
   * @param defaultValue the default value to return if the key is not found or the value is not a
   *     Map
   * @return the Map value or the default value
   */
  public static Map<String, Object> getMap(
      Map<String, Object> map, String key, Map<String, Object> defaultValue) {
    if (map == null || !map.containsKey(key)) {
      return defaultValue;
    }
    Object value = map.get(key);
    if (value == null) {
      return defaultValue;
    }
    if (value instanceof Map) {
      @SuppressWarnings("unchecked")
      Map<String, Object> result = (Map<String, Object>) value;
      return result;
    }
    return defaultValue;
  }

  /**
   * Serializes an object to a JSON string.
   *
   * @param obj the object to serialize
   * @return the JSON string representation of the object, or null if serialization fails
   */
  public static String toJson(Object obj) {
    if (obj == null) {
      return null;
    }
    try {
      return MAPPER.writeValueAsString(obj);
    } catch (JsonProcessingException e) {
      return null;
    }
  }

  /**
   * Deserializes a JSON string to an object of the specified class.
   *
   * @param <T> the type of the object to deserialize to
   * @param json the JSON string to deserialize
   * @param clazz the class of the object to deserialize to
   * @return the deserialized object, or null if deserialization fails
   */
  public static <T> T fromJson(String json, Class<T> clazz) {
    if (json == null || json.isEmpty()) {
      return null;
    }
    try {
      return MAPPER.readValue(json, clazz);
    } catch (IOException e) {
      return null;
    }
  }

  /**
   * Deserializes a JSON string to an object using the specified TypeReference.
   *
   * @param <T> the type of the object to deserialize to
   * @param json the JSON string to deserialize
   * @param typeReference the TypeReference describing the type to deserialize to
   * @return the deserialized object, or null if deserialization fails
   */
  public static <T> T fromJson(String json, TypeReference<T> typeReference) {
    if (json == null || json.isEmpty()) {
      return null;
    }
    try {
      return MAPPER.readValue(json, typeReference);
    } catch (IOException e) {
      return null;
    }
  }

  /**
   * Deserializes a JSON string to a String array.
   *
   * @param json the JSON string to deserialize
   * @return a String array, or an empty array if deserialization fails
   */
  public static String[] fromJsonToStringArray(String json) {
    if (json == null || json.isEmpty()) {
      return new String[0];
    }
    try {
      return MAPPER.readValue(json, String[].class);
    } catch (IOException e) {
      return new String[0];
    }
  }

  /**
   * Deserializes a JSON string to an object of the specified class and sets its rawJson field.
   *
   * @param <T> the type of the object to deserialize to, must implement HasRawJson
   * @param json the JSON string to deserialize
   * @param clazz the class of the object to deserialize to
   * @return the deserialized object with rawJson set, or null if deserialization fails
   */
  public static <T extends HasRawJson> T fromJsonWithRawJson(String json, Class<T> clazz) {
    if (json == null || json.isEmpty()) {
      return null;
    }
    try {
      T obj = MAPPER.readValue(json, clazz);
      if (obj != null) {
        obj.setRawJson(json);
      }
      return obj;
    } catch (IOException e) {
      return null;
    }
  }

  /**
   * Deserializes a JSON string to a Map&lt;String, Object&gt;.
   *
   * @param json the JSON string to deserialize
   * @param defaultValue the default value to return if deserialization fails
   * @return the deserialized map, or the defaultValue if deserialization fails
   */
  public static Map<String, Object> fromJsonToMap(String json, Map<String, Object> defaultValue) {
    if (json == null || json.isEmpty()) {
      return defaultValue;
    }
    try {
      Map<String, Object> map = MAPPER.readValue(json, new TypeReference<Map<String, Object>>() {});
      return map != null ? map : defaultValue;
    } catch (IOException e) {
      return defaultValue;
    }
  }

  /**
   * Deserializes a JSON string to an Object array.
   *
   * @param json the JSON string to deserialize
   * @param defaultValue the default value to return if deserialization fails
   * @return the deserialized array, or the defaultValue if deserialization fails
   */
  public static Object[] fromJsonToArray(String json, Object[] defaultValue) {
    if (json == null || json.isEmpty()) {
      return defaultValue;
    }
    try {
      Object[] array = MAPPER.readValue(json, Object[].class);
      return array != null ? array : defaultValue;
    } catch (IOException e) {
      return defaultValue;
    }
  }

  private static Object nodeToObject(JsonNode node) {
    if (node.isTextual()) {
      return node.asText();
    } else if (node.isNumber()) {
      return node.asDouble();
    } else if (node.isBoolean()) {
      return node.asBoolean();
    } else if (node.isNull()) {
      return null;
    } else if (node.isObject()) {
      try {
        return MAPPER.convertValue(node, new TypeReference<Map<String, Object>>() {});
      } catch (Exception e) {
        return null;
      }
    } else if (node.isArray()) {
      ArrayNode array = (ArrayNode) node;
      Object[] result = new Object[array.size()];
      for (int i = 0; i < array.size(); i++) {
        result[i] = nodeToObject(array.get(i));
      }
      return result;
    }
    return null;
  }

  private static class MapDeserializer extends JsonDeserializer<Map<String, Object>> {
    @Override
    public Map<String, Object> deserialize(
        com.fasterxml.jackson.core.JsonParser jp, DeserializationContext ctxt) throws IOException {
      JsonNode node = jp.getCodec().readTree(jp);
      Map<String, Object> result = new HashMap<>();
      if (node.isObject()) {
        ObjectNode objectNode = (ObjectNode) node;
        objectNode
            .fields()
            .forEachRemaining(
                entry -> {
                  result.put(entry.getKey(), nodeToObject(entry.getValue()));
                });
      }
      return result;
    }
  }
}
