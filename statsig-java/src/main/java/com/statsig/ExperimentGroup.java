package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.Map;

public class ExperimentGroup {
  public final String groupName;
  public final Map<String, Object> returnValue;

  @JSONCreator
  ExperimentGroup(
      @JSONField(name = "group_name") String groupName,
      @JSONField(name = "return_value") Map<String, Object> returnValue) {
    this.groupName = groupName;
    this.returnValue = returnValue;
  }

  public String getGroupName() {
    return groupName;
  }

  public Map<String, Object> getReturnValue() {
    return returnValue;
  }
}
