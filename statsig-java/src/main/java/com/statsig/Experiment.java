package com.statsig;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.core.type.TypeReference;
import com.statsig.internal.JacksonUtil;
import java.util.List;
import java.util.Map;

public class Experiment extends BaseConfig {
  @JsonProperty("group_name")
  public final String groupName;

  public List<Map<String, String>> secondaryExposures;

  public Experiment() {
    super();
    this.groupName = null;
  }

  Experiment(
      String name,
      Map<String, Object> value,
      String ruleID,
      EvaluationDetails evaluationDetails,
      String idType,
      String groupName,
      List<Map<String, String>> secondaryExposures) {
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
      Experiment experiment = JacksonUtil.fromJsonWithRawJson(json, Experiment.class);
      if (experiment == null) {
        return null;
      }

      Map<String, Object> rootMap =
          JacksonUtil.fromJson(json, new TypeReference<Map<String, Object>>() {});
      List<Map<String, String>> secondaryExposures = null;
      if (rootMap != null && rootMap.containsKey("__evaluation")) {
        Map<String, Object> evaluation = (Map<String, Object>) rootMap.get("__evaluation");
        if (evaluation != null && evaluation.containsKey("secondary_exposures")) {
          secondaryExposures =
              JacksonUtil.getObjectMapper()
                  .convertValue(
                      evaluation.get("secondary_exposures"),
                      new TypeReference<List<Map<String, String>>>() {});
        }
      }

      if (secondaryExposures != null) {
        experiment.secondaryExposures = secondaryExposures;
      }

      return experiment;
    } catch (Exception e) {
      System.err.println("Error deserializing Experiment: " + e.getMessage());
      e.printStackTrace();
      return null;
    }
  }
}
