package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.statsig.internal.HasRawJson;

@JsonIgnoreProperties(ignoreUnknown = true)
public class FeatureGate implements HasRawJson {
  public String name;
  public boolean value;

  @JsonProperty("rule_id")
  public String ruleID;

  @JsonProperty("details")
  public EvaluationDetails evaluationDetails;

  @JsonProperty("id_type")
  public String idType;

  @JsonIgnore String rawJson;

  public FeatureGate() {}

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

  public void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }
}
