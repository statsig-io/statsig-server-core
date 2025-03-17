package com.statsig;

import com.google.gson.JsonElement;

import java.util.Map;

public class DynamicConfig extends BaseConfig {
    DynamicConfig(String name, Map<String, JsonElement> value, String ruleID, EvaluationDetails evaluationDetails,
            String idType) {
        super(name, value, ruleID, evaluationDetails, idType);
    }
}
