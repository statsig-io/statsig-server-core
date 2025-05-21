package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

@JsonIgnoreProperties(ignoreUnknown = true)
public class CMABRankedVariant {
  @JsonProperty("variant_name")
  public String variantName;

  @JsonProperty("rule_id")
  public String ruleID;

  @JsonProperty("value")
  public Map<String, Object> value;

  @JsonProperty("score")
  public double score;

  @JsonProperty("cmab_name")
  public String cmabName;

  public CMABRankedVariant() {}

  CMABRankedVariant(
      String variantName, Map<String, Object> value, String ruleID, double score, String cmabName) {
    this.variantName = variantName;
    this.value = value;
    this.ruleID = ruleID;
    this.score = score;
    this.cmabName = cmabName;
  }
}
