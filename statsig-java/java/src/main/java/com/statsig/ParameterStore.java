package com.statsig;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

public class ParameterStore {
  public String name;

  @JsonProperty("details")
  public EvaluationDetails evaluationDetails;

  private Statsig statsigInstance;
  private StatsigUser user;

  ParameterStore(String name, EvaluationDetails evaluationDetails) {
    this.name = name;
    this.evaluationDetails = evaluationDetails;
  }

  public ParameterStore() {}

  public String getName() {
    return name;
  }

  public EvaluationDetails getEvaluationDetails() {
    return evaluationDetails;
  }

  void setStatsigInstance(Statsig statsigInstance) {
    this.statsigInstance = statsigInstance;
  }

  void setUser(StatsigUser user) {
    this.user = user;
  }

  public String getString(String parameterName, String defaultValue) {
    String value =
        statsigInstance.getStringFromParameterStore(user, name, parameterName, defaultValue);
    if (value == null) {
      return defaultValue;
    }
    return value;
  }

  public boolean getBoolean(String parameterName, boolean defaultValue) {
    return statsigInstance.getBooleanFromParameterStore(user, name, parameterName, defaultValue);
  }

  public double getDouble(String parameterName, double defaultValue) {
    return statsigInstance.getDoubleFromParameterStore(user, name, parameterName, defaultValue);
  }

  public int getInt(String parameterName, int defaultValue) {
    return statsigInstance.getIntFromParameterStore(user, name, parameterName, defaultValue);
  }

  public long getLong(String parameterName, long defaultValue) {
    return statsigInstance.getLongFromParameterStore(user, name, parameterName, defaultValue);
  }

  public Object[] getArray(String parameterName, Object[] defaultValue) {
    return statsigInstance.getArrayFromParameterStore(user, name, parameterName, defaultValue);
  }

  public Map<String, Object> getMap(String parameterName, Map<String, Object> defaultValue) {
    return statsigInstance.getMapFromParameterStore(user, name, parameterName, defaultValue);
  }
}
