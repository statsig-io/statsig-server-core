package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.List;
import java.util.Map;

public class StickyValues {
  @JSONField(name = "value")
  public final boolean value;

  @JSONField(name = "json_value")
  public final Map<String, Object> jsonValue;

  @JSONField(name = "rule_id")
  public final String ruleId;

  @JSONField(name = "group_name")
  public final String groupName;

  @JSONField(name = "secondary_exposures")
  public final List<Map<String, String>> secondaryExposures;

  @JSONField(name = "undelegated_secondary_exposures")
  public final List<Map<String, String>> undelegatedSecondaryExposures;

  @JSONField(name = "config_delegate")
  public final String configDelegate;

  @JSONField(name = "explicit_parameters")
  public final List<String> explicitParameters;

  @JSONField(name = "time")
  public final Long time;

  @JSONField(name = "config_version")
  public final Integer configVersion;

  @JSONCreator
  public StickyValues(
      @JSONField(name = "value") boolean value,
      @JSONField(name = "json_value") Map<String, Object> jsonValue,
      @JSONField(name = "rule_id") String ruleId,
      @JSONField(name = "group_name") String groupName,
      @JSONField(name = "secondary_exposures") List<Map<String, String>> secondaryExposures,
      @JSONField(name = "undelegated_secondary_exposures")
          List<Map<String, String>> undelegatedSecondaryExposures,
      @JSONField(name = "config_delegate") String configDelegate,
      @JSONField(name = "explicit_parameters") List<String> explicitParameters,
      @JSONField(name = "time") Long time,
      @JSONField(name = "config_version") Integer configVersion) {
    this.value = value;
    this.jsonValue = jsonValue;
    this.ruleId = ruleId;
    this.groupName = groupName;
    this.secondaryExposures = secondaryExposures;
    this.undelegatedSecondaryExposures = undelegatedSecondaryExposures;
    this.configDelegate = configDelegate;
    this.explicitParameters = explicitParameters;
    this.time = time;
    this.configVersion = configVersion;
  }

  public boolean getValue() {
    return value;
  }

  public Map<String, Object> getJsonValue() {
    return jsonValue;
  }

  public String getRuleId() {
    return ruleId;
  }

  public String getGroupName() {
    return groupName;
  }

  public List<Map<String, String>> getSecondaryExposures() {
    return secondaryExposures;
  }

  public List<Map<String, String>> getUndelegatedSecondaryExposures() {
    return undelegatedSecondaryExposures;
  }

  public String getConfigDelegate() {
    return configDelegate;
  }

  public List<String> getExplicitParameters() {
    return explicitParameters;
  }

  public Long getTime() {
    return time;
  }

  public Integer getConfigVersion() {
    return configVersion;
  }
}
