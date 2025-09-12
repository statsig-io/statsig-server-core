package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import com.statsig.internal.HasRawJson;

public class FeatureGate implements HasRawJson {
  public final String name;
  public final boolean value;
  public final String ruleID;
  public final EvaluationDetails evaluationDetails;
  public final String idType;

  @JSONField(deserialize = false)
  String rawJson;

  @JSONCreator
  FeatureGate(
      @JSONField(name = "name") String name,
      @JSONField(name = "value") boolean value,
      @JSONField(name = "rule_id") String ruleID,
      @JSONField(name = "details") EvaluationDetails evaluationDetails,
      @JSONField(name = "id_type") String idType) {
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
