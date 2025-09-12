package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.Map;

public class CMABRankedVariant {
  public final String variantName;
  public final String ruleID;
  public final Map<String, Object> value;
  public final double score;
  public final String cmabName;

  @JSONCreator
  CMABRankedVariant(
      @JSONField(name = "variant_name") String variantName,
      @JSONField(name = "value") Map<String, Object> value,
      @JSONField(name = "rule_id") String ruleID,
      @JSONField(name = "score") double score,
      @JSONField(name = "cmab_name") String cmabName) {
    this.variantName = variantName;
    this.value = value;
    this.ruleID = ruleID;
    this.score = score;
    this.cmabName = cmabName;
  }
}
