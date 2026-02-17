package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.Map;

public class DynamicConfig extends BaseConfig {
  @JSONCreator
  DynamicConfig(
      @JSONField(name = "name") String name,
      @JSONField(name = "value") Map<String, Object> value,
      @JSONField(name = "ruleID") String ruleID,
      @JSONField(name = "details") EvaluationDetails evaluationDetails,
      @JSONField(name = "idType") String idType) {
    super(name, value, ruleID, evaluationDetails, idType);
  }
}
