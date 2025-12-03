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

  public String getString(String key, String fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    if (!(val instanceof String)) {
      return fallback;
    }
    logLayerExposure(key);
    return (String) val;
  }

  public boolean getBoolean(String key, boolean fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    if (!(val instanceof Boolean)) {
      return fallback;
    }
    logLayerExposure(key);
    return (boolean) val;
  }

  public double getDouble(String key, double fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    // If val is not a Number type, return fallback (type mismatch)
    if (!(val instanceof Number)) {
      return fallback;
    }
    logLayerExposure(key);
    return ((Number) val).doubleValue();
  }

  public int getInt(String key, int fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    // If val is not a Number type, return fallback (type mismatch)
    if (!(val instanceof Number)) {
      return fallback;
    }
    logLayerExposure(key);
    return ((Number) val).intValue();
  }

  public long getLong(String key, long fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    // If val is not a Number type, return fallback (type mismatch)
    if (!(val instanceof Number)) {
      return fallback;
    }
    logLayerExposure(key);
    return ((Number) val).longValue();
  }

  public Object[] getArray(String key, Object[] fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    // If val is not an array or list type, return fallback (type mismatch)
    if (!(val instanceof Object[]) && !(val instanceof List<?>)) {
      return fallback;
    }
    logLayerExposure(key);
    if (val instanceof Object[]) {
      return (Object[]) val;
    }
    return ((List<?>) val).toArray(new Object[0]);
  }

  public Map<String, Object> getMap(String key, Map<String, Object> fallback) {
    Object val = value.get(key);
    if (val == null) {
      return fallback;
    }
    // If val is not a Map type, return fallback (type mismatch)
    if (!(val instanceof Map)) {
      return fallback;
    }
    logLayerExposure(key);
    @SuppressWarnings("unchecked")
    Map<String, Object> map = (Map<String, Object>) val;
    return map;
  }

  private void logLayerExposure(String key) {
    if (statsigInstance != null && rawJson != null && !disableExposureLogging) {
      statsigInstance.logLayerParamExposure(rawJson, key);
    }
  }
}
