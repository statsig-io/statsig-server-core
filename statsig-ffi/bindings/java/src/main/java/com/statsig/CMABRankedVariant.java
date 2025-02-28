package com.statsig;

import java.util.Map;

import com.google.gson.JsonElement;
import com.google.gson.annotations.SerializedName;

public class CMABRankedVariant {
    @SerializedName("variant_name")
    public final String variantName;
    @SerializedName("rule_id")
    public final String ruleID;
    @SerializedName("value")
    public final Map<String, JsonElement> value;
    public final double score;
    @SerializedName("cmab_name")
    public final String cmabName;

    CMABRankedVariant(String variantName, Map<String, JsonElement> value, String ruleID, double score,
            String cmabName) {
        this.variantName = variantName;
        this.value = value;
        this.ruleID = ruleID;
        this.score = score;
        this.cmabName = cmabName;
    }
}
