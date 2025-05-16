package com.statsig;

import com.google.gson.JsonElement;
import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;
import com.statsig.internal.GsonUtil;
import java.util.Map;

public abstract class BaseConfig {
  public final String name;

  @SerializedName("rule_id")
  public final String ruleID;

  @SerializedName("value")
  public final Map<String, JsonElement> value;

  @SerializedName("details")
  public final EvaluationDetails evaluationDetails;

  @SerializedName("id_type")
  public final String idType;

  @Expose(serialize = false, deserialize = false)
  protected String rawJson;

  protected BaseConfig(
      String name,
      Map<String, JsonElement> value,
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

  public Map<String, JsonElement> getValue() {
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
  void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }

  // get parameters
  public String getString(String key, String defaultValue) {
    return GsonUtil.getString(value, key, defaultValue);
  }

  public boolean getBoolean(String key, boolean defaultValue) {
    return GsonUtil.getBoolean(value, key, defaultValue);
  }

  public double getDouble(String key, double defaultValue) {
    return GsonUtil.getDouble(value, key, defaultValue);
  }

  public int getInt(String key, int defaultValue) {
    return GsonUtil.getInt(value, key, defaultValue);
  }

  public long getLong(String key, long defaultValue) {
    return GsonUtil.getLong(value, key, defaultValue);
  }

  public Object[] getArray(String key, Object[] defaultValue) {
    return GsonUtil.getArray(value, key, defaultValue);
  }

  public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
    return GsonUtil.getMap(value, key, defaultValue);
  }
}
