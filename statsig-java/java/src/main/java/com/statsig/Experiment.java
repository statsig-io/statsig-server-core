package com.statsig;

import com.google.gson.*;
import com.google.gson.annotations.SerializedName;
import com.google.gson.reflect.TypeToken;
import java.util.List;
import java.util.Map;

public class Experiment extends BaseConfig {
  @SerializedName("group_name")
  public final String groupName;

  public List<Map<String, String>> secondaryExposures;

  Experiment(
      String name,
      Map<String, JsonElement> value,
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
    Gson gson = new Gson();
    JsonObject jsonObject = JsonParser.parseString(json).getAsJsonObject();

    List<Map<String, String>> secondaryExposures = null;

    if (jsonObject.has("__evaluation")) {
      JsonObject evaluation = jsonObject.getAsJsonObject("__evaluation");
      secondaryExposures =
          gson.fromJson(
              evaluation.get("secondary_exposures"),
              new TypeToken<List<Map<String, String>>>() {}.getType());
    }

    Experiment experiment = gson.fromJson(json, Experiment.class);
    experiment.secondaryExposures = secondaryExposures;
    experiment.setRawJson(json);
    return experiment;
  }
}
