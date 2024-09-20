package com.statsig;

import java.util.HashMap;
import java.util.Map;

public class EvaluationDetails {
    /**
     * last config updated time
     */
    public long lcut;

    /**
     * The time when we received this config
     */
    public long receivedAt;

    /**
     * Evaluation reason
     */
    public String reason;

    EvaluationDetails(long lcut, long receivedAt, String reason) {
        this.lcut = lcut;
        this.receivedAt = receivedAt;
        this.reason = reason;
    }

    @Override
    public String toString() {
        return String.format(
                "EvaluationDetails { lcut=%s, receivedAt=%s, reason='%s' }",
                lcut, receivedAt, reason
        );
    }

    public Map<String, Object> toMap() {
        Map<String, Object> map = new HashMap<>();
        map.put("lcut", lcut);
        map.put("receivedAt", receivedAt);
        map.put("reason", reason);
        return map;
    }
}
