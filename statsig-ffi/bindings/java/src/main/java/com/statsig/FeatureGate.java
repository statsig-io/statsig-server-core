package com.statsig;

import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;

public class FeatureGate {
    public String name;
    public boolean value;

    @SerializedName("rule_id")
    public String ruleID;

    @SerializedName("details")
    public EvaluationDetails evaluationDetails;

    @Expose(serialize = false, deserialize = false)
    String rawJson;

    FeatureGate(String name, boolean value, String ruleID, EvaluationDetails evaluationDetails
    ) {
        this.name = name;
        this.value = value;
        this.ruleID = ruleID;
        this.evaluationDetails = evaluationDetails;
    }

    public String getRawJson() {
        return rawJson;
    }

    void setRawJson(String rawJson) {
        this.rawJson = rawJson;
    }
}


