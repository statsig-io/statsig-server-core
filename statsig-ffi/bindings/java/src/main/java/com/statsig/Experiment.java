package com.statsig;

import java.util.Map;

public class Experiment {
    public final String name;
    public final String ruleID;
    public final Map<String, Object> value;

    public Experiment(String name, String ruleID, Map<String, Object> value) {
        this.name = name;
        this.ruleID = ruleID;
        this.value = value;
    }
}
