package com.statsig;

import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;

public class FeatureGate {
  public String name;
  public boolean value;

  @SerializedName("rule_id")
  public String ruleID;

  @SerializedName("details")
  public EvaluationDetails evaluationDetails;

  @SerializedName("id_type")
  public String idType;

  @Expose(serialize = false, deserialize = false)
  String rawJson;

  FeatureGate(
      String name,
      boolean value,
      String ruleID,
      EvaluationDetails evaluationDetails,
      String idType) {
    this.name = name;
    this.value = value;
    this.ruleID = ruleID;
    this.evaluationDetails = evaluationDetails;
    this.idType = idType;
  }

  public String getName() {
    return name;
  }

  public boolean getValue() {
    return value;
  }

  public String getRuleID() {
    return ruleID;
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

  void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }
}
