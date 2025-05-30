package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.statsig.internal.HasRawJson;
import java.util.List;
import java.util.Map;

@JsonIgnoreProperties(ignoreUnknown = true)
public abstract class BaseConfig implements HasRawJson {
  public final String name;

  @JsonProperty("rule_id")
  public final String ruleID;

  @JsonProperty("value")
  public final Map<String, Object> value;

  @JsonProperty("details")
  public final EvaluationDetails evaluationDetails;

  @JsonProperty("id_type")
  public final String idType;

  @JsonIgnore protected String rawJson;

  protected BaseConfig() {
    this.name = null;
    this.value = null;
    this.ruleID = null;
    this.evaluationDetails = null;
    this.idType = null;
  }

  protected BaseConfig(
      String name,
      Map<String, Object> value,
      String ruleID,
      EvaluationDetails evaluationDetails,
      String idType) {
    this.name = name;
    this.value = value;
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
