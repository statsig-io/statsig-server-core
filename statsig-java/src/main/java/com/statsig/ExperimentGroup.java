package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.Map;

public class ExperimentGroup {
  public final String groupName;
  public final String ruleID;
  public final String idType;
  public final Map<String, Object> returnValue;

  @JSONCreator
  ExperimentGroup(
      @JSONField(name = "group_name") String groupName,
      @JSONField(name = "rule_id") String ruleID,
      @JSONField(name = "id_type") String idType,
      @JSONField(name = "return_value") Map<String, Object> returnValue) {
    this.groupName = groupName;
    this.ruleID = ruleID;
    this.idType = idType;
    this.returnValue = returnValue;
  }

  public String getGroupName() {
    return groupName;
  }

  public String getRuleID() {
    return ruleID;
  }

  public String getIDType() {
    return idType;
  }

  public Map<String, Object> getReturnValue() {
    return returnValue;
  }
}
