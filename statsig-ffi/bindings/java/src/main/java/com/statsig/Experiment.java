package com.statsig;

import java.util.Map;

import com.google.gson.JsonElement;
import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;
import com.statsig.internal.GsonUtil;

public class Experiment {
    public final String name;
    @SerializedName("rule_id")
    public final String ruleID;
    @SerializedName("value")
    public final Map<String, JsonElement> value;
    @SerializedName("group_name")
    public final String groupName;
    @SerializedName("details")
    public final EvaluationDetails evaluationDetails;
    @Expose(serialize = false, deserialize = false)
    String rawJson;

    Experiment(String name, Map<String, JsonElement> value, String ruleID, String groupName,
                      EvaluationDetails evaluationDetails) {
        this.name = name;
        this.value = value;
        this.ruleID = ruleID;
        this.groupName = groupName;
        this.evaluationDetails = evaluationDetails;
    }

    public String getRawJson() {
        return rawJson;
    }

    void setRawJson(String rawJson) {
        this.rawJson = rawJson;
    }

    public String getString(String key, String defaultValue) {
        return GsonUtil.getString(value, key, defaultValue);
    }

    public boolean getBoolean(String key, boolean defaultValue) {
        return GsonUtil.getBoolean(value, key, defaultValue);
    }

    public double getDouble(String key, double defaultValue) {
        return GsonUtil.getDouble(value, key, defaultValue);
    }

    public int getInt(String key, int defaultValue) {
        return GsonUtil.getInt(value, key, defaultValue);
    }

    public long getLong(String key, long defaultValue) {
        return GsonUtil.getLong(value, key, defaultValue);
    }

    public Object[] getArray(String key, Object[] defaultValue) {
        return GsonUtil.getArray(value, key, defaultValue);
    }

    public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
        return GsonUtil.getMap(value, key, defaultValue);
    }
}
