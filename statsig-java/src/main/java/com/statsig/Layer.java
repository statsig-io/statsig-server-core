package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.statsig.internal.HasRawJson;
import java.util.Map;

@JsonIgnoreProperties(ignoreUnknown = true)
public class Layer implements HasRawJson {
  public String name;

  @JsonProperty("rule_id")
  public String ruleID;

  @JsonProperty("group_name")
  public String groupName;

  @JsonProperty("__value")
  public Map<String, Object> value;

  @JsonProperty("allocated_experiment_name")
  public String allocatedExperimentName;

  @JsonProperty("details")
  public EvaluationDetails evaluationDetails;

  @JsonIgnore String rawJson;

  private Statsig statsigInstance;
  private boolean disableExposureLogging;

  public Layer() {}

  Layer(
      String name,
      String ruleID,
      String groupName,
      Map<String, Object> value,
      String allocatedExperimentName,
      EvaluationDetails evaluationDetails) {
    this.name = name;
    this.ruleID = ruleID;
    this.groupName = groupName;
    this.value = value;
    this.evaluationDetails = evaluationDetails;
    this.allocatedExperimentName = allocatedExperimentName;
  }

  public String getName() {
    return name;
  }

  public String getRuleID() {
    return ruleID;
  }

  public String getGroupName() {
    return groupName;
  }

  public Map<String, Object> getValue() {
    return value;
  }

  public String getAllocatedExperimentName() {
    return allocatedExperimentName;
  }

  public EvaluationDetails getEvaluationDetails() {
    return evaluationDetails;
  }

  public String getRawJson() {
    return rawJson;
  }

  public void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }

  void setStatsigInstance(Statsig statsigInstance) {
    this.statsigInstance = statsigInstance;
  }

  void setDisableExposureLogging(boolean disableExposureLogging) {
    this.disableExposureLogging = disableExposureLogging;
  }

  public String getString(String key, String defaultValue) {
    logLayerExposure(key);
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
    logLayerExposure(key);
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
    logLayerExposure(key);
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
    logLayerExposure(key);
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
    logLayerExposure(key);
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
    logLayerExposure(key);
    Object val = value.get(key);
    if (val == null) {
      return defaultValue;
    }
    if (val instanceof Object[]) {
      return (Object[]) val;
    }
    return defaultValue;
  }

  public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
    logLayerExposure(key);
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

  private void logLayerExposure(String key) {
    if (statsigInstance != null && rawJson != null && !disableExposureLogging) {
      statsigInstance.logLayerParamExposure(rawJson, key);
    }
  }
}
