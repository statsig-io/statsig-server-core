package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import com.statsig.internal.HasRawJson;
import java.util.List;
import java.util.Map;

public class Layer implements HasRawJson {
  public final String name;

  public final String ruleID;

  public final String groupName;

  public final Map<String, Object> value;

  public final String allocatedExperimentName;

  public final EvaluationDetails evaluationDetails;

  @JSONField(deserialize = false)
  String rawJson;

  private Statsig statsigInstance;
  private boolean disableExposureLogging;

  @JSONCreator
  Layer(
      @JSONField(name = "name") String name,
      @JSONField(name = "rule_id") String ruleID,
      @JSONField(name = "group_name") String groupName,
      @JSONField(name = "__value") Map<String, Object> value,
      @JSONField(name = "allocated_experiment_name") String allocatedExperimentName,
      @JSONField(name = "details") EvaluationDetails evaluationDetails) {
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
    if (val instanceof List<?>) {
      return ((List<?>) val).toArray(new Object[0]);
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
