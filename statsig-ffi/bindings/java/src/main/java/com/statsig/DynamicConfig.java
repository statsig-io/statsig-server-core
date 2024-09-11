package com.statsig;

import com.google.gson.JsonElement;
import com.google.gson.annotations.SerializedName;
import com.statsig.internal.GsonUtil;

import java.util.Map;

public class DynamicConfig extends Experiment {

    @SerializedName("value")
    public final Map<String, JsonElement> dynamicValue;

    DynamicConfig(String name, Map<String, JsonElement> value, String ruleID, EvaluationDetails evaluationDetails) {
        super(name, value, ruleID, null, evaluationDetails);
        this.dynamicValue = value;
    }

    @Override
    public String getString(String key, String defaultValue) {
        return GsonUtil.getString(dynamicValue, key, defaultValue);
    }

    @Override
    public boolean getBoolean(String key, boolean defaultValue) {
        return GsonUtil.getBoolean(dynamicValue, key, defaultValue);
    }

    @Override
    public double getDouble(String key, double defaultValue) {
        return GsonUtil.getDouble(dynamicValue, key, defaultValue);
    }

    @Override
    public int getInt(String key, int defaultValue) {
        return GsonUtil.getInt(dynamicValue, key, defaultValue);
    }

    @Override
    public long getLong(String key, long defaultValue) {
        return GsonUtil.getLong(dynamicValue, key, defaultValue);
    }

    @Override
    public Object[] getArray(String key, Object[] defaultValue) {
        return GsonUtil.getArray(dynamicValue, key, defaultValue);
    }

    @Override
    public Map<String, Object> getMap(String key, Map<String, Object> defaultValue) {
        return GsonUtil.getMap(dynamicValue, key, defaultValue);
    }
}
