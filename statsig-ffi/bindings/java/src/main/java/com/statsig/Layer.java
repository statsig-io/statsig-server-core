package com.statsig;

import java.util.Map;

import com.google.gson.JsonElement;
import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;
import com.statsig.internal.GsonUtil;

public class Layer {
    public String name;
    @SerializedName("rule_id")
    public String ruleID;
    @SerializedName("group_name")
    public String groupName;
    @SerializedName("__value")
    public Map<String, JsonElement> value;
    @SerializedName("allocated_experiment_name")
    public String allocatedExperimentName;
    @SerializedName("details")
    public EvaluationDetails evaluationDetails;
    @Expose(serialize = false, deserialize = false)
    String rawJson;

    private Statsig statsigInstance;

    Layer(String name, String ruleID, String groupName, Map<String, JsonElement> value,
                 String allocatedExperimentName, EvaluationDetails evaluationDetails) {
        this.name = name;
        this.ruleID = ruleID;
        this.groupName = groupName;
        this.value = value;
        this.evaluationDetails = evaluationDetails;
        this.allocatedExperimentName = allocatedExperimentName;
    }

    public String getName() {
        return name;
    }

    public String getRuleID() {
        return ruleID;
    }

    public String getGroupName() {
        return groupName;
    }

    public Map<String, JsonElement> getValue() {
        return value;
    }

    public String getAllocatedExperimentName() {
        return allocatedExperimentName;
    }

    public EvaluationDetails getEvaluationDetails() {
        return evaluationDetails;
    }

    public String getRawJson() {
        return rawJson;
    }

    void setRawJson(String rawJson) {
        this.rawJson = rawJson;
    }

    void setStatsigInstance(Statsig statsigInstance) {
        this.statsigInstance = statsigInstance;
    }

    public String getString(String key, String defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getString(value, key, defaultValue);
    }

    public boolean getBoolean(String key, boolean defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getBoolean(value, key, defaultValue);
    }

    public double getDouble(String key, double defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getDouble(value, key, defaultValue);
    }

    public int getInt(String key, int defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getInt(value, key, defaultValue);
    }

    public long getLong(String key, long defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getLong(value, key, defaultValue);
    }

    public Object[] getArray(String key, Object[] defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getArray(value, key, defaultValue);
    }

    public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
        logLayerExposure(key);
        return GsonUtil.getMap(value, key, defaultValue);
    }

    private void logLayerExposure(String key) {
        if (statsigInstance != null && rawJson != null) {
            statsigInstance.logLayerParamExposure(rawJson, key);
        }
    }
}
