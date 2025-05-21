package com.statsig;

import java.util.Map;

public class DynamicConfig extends BaseConfig {
  DynamicConfig(
      String name,
      Map<String, Object> value,
      String ruleID,
      EvaluationDetails evaluationDetails,
      String idType) {
    super(name, value, ruleID, evaluationDetails, idType);
  }

  public DynamicConfig() {
    super();
  }
}
