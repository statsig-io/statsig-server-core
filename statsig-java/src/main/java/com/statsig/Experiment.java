package com.statsig;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONObject;
import com.alibaba.fastjson2.TypeReference;
import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.List;
import java.util.Map;

public class Experiment extends BaseConfig {
  public final String groupName;
  public List<Map<String, String>> secondaryExposures;

  @JSONCreator
  Experiment(
      @JSONField(name = "name") String name,
      @JSONField(name = "value") Map<String, Object> value,
      @JSONField(name = "ruleID") String ruleID,
      @JSONField(name = "details") EvaluationDetails evaluationDetails,
      @JSONField(name = "idType") String idType,
      @JSONField(name = "groupName") String groupName,
      @JSONField(name = "secondaryExposures") List<Map<String, String>> secondaryExposures) {
    super(name, value, ruleID, evaluationDetails, idType);
    this.groupName = groupName;
    this.secondaryExposures = secondaryExposures;
  }

  public String getGroupName() {
    return groupName;
  }

  public List<Map<String, String>> getSecondaryExposures() {
    return secondaryExposures;
  }

  static Experiment fromJson(String json) {
    if (json == null || json.isEmpty()) {
      return null;
    }

    try {
      Experiment experiment = JSON.parseObject(json, Experiment.class);
      if (experiment == null) {
        return null;
      }

      experiment.setRawJson(json);

      JSONObject root = JSON.parseObject(json);
      if (root != null) {
        Object rawSecondaryExposures = root.get("secondaryExposures");
        if (rawSecondaryExposures != null) {
          List<Map<String, String>> se =
              JSON.parseObject(
                  rawSecondaryExposures.toString(),
                  new TypeReference<List<Map<String, String>>>() {});
          experiment.secondaryExposures = se;
        }
      }
      return experiment;
    } catch (Exception e) {
      System.err.println("Error deserializing Experiment: " + e.getMessage());
      e.printStackTrace();
      return null;
    }
  }
}
