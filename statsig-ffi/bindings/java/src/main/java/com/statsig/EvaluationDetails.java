package com.statsig;

public class EvaluationDetails {
    public long lcut;
    public long receivedAt;
    public String reason;

    EvaluationDetails(long lcut, long receivedAt, String reason) {
        this.lcut = lcut;
        this.receivedAt = receivedAt;
        this.reason = reason;
    }
}
