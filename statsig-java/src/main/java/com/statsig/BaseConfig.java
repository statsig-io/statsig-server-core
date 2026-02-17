package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import com.statsig.internal.HasRawJson;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public abstract class BaseConfig implements HasRawJson {
  public final String name;
  public final String ruleID;
  public final Map<String, Object> value;
  public final EvaluationDetails evaluationDetails;
  public final String idType;
  protected String rawJson;

  @JSONCreator
  protected BaseConfig(
      @JSONField(name = "name") String name,
      @JSONField(name = "value") Map<String, Object> value,
      @JSONField(name = "ruleID") String ruleID,
      @JSONField(name = "details") EvaluationDetails evaluationDetails,
      @JSONField(name = "idType") String idType) {
    this.name = name;
    this.value = value == null ? new HashMap<>() : value;
    this.ruleID = ruleID;
    this.evaluationDetails = evaluationDetails;
    this.idType = idType;
  }

  // Getters
  public String getName() {
    return name;
  }

  public String getRuleID() {
    return ruleID;
  }

  public Map<String, Object> getValue() {
    return value;
  }

  public EvaluationDetails getEvaluationDetails() {
    return evaluationDetails;
  }

  public String getIDType() {
    return idType;
  }

  public String getRawJson() {
    return rawJson;
  }

  // Setter for rawJson
  public void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }

  // get parameters
  public String getString(String key, String defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof String) {
      return (String) val;
    }
    return defaultValue;
  }

  public boolean getBoolean(String key, boolean defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Boolean) {
      return (Boolean) val;
    }
    return defaultValue;
  }

  public double getDouble(String key, double defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Number) {
      return ((Number) val).doubleValue();
    }
    return defaultValue;
  }

  public int getInt(String key, int defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Number) {
      return ((Number) val).intValue();
    }
    return defaultValue;
  }

  public long getLong(String key, long defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Number) {
      return ((Number) val).longValue();
    }
    return defaultValue;
  }

  public Object[] getArray(String key, Object[] defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Object[]) {
      return (Object[]) val;
    }
    if (val instanceof List) {
      List<?> list = (List<?>) val;
      return list.toArray(new Object[0]);
    }
    return defaultValue;
  }

  public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Map) {
      @SuppressWarnings("unchecked")
      Map<String, Object> map = (Map<String, Object>) val;
      return map;
    }
    return defaultValue;
  }
}
